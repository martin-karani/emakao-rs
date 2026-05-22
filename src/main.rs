use std::sync::Arc;

use apalis::prelude::WorkerBuilder;
use apalis_redis::RedisStorage;

use emakao::{
    config::Config,
    infrastructure::{
        audit::AuditLogger,
        cache::settings_cache::run_invalidation_listener,
        cache::{EntitlementCache, PermissionCache, SettingsCache},
        jobs::{
            archival::{process_archival, ArchivalJob},
            workflow::{execute_workflow_job, WorkflowEngine, WorkflowJob},
        },
        notifications::dispatcher::NotificationDispatcher,
        providers::{africas_talking::AfricasTalkingProvider, registry::ProviderRegistry},
        scheduler::billing_worker::{build_billing_monitor, BillingContext},
    },
    presentation::{app_state::AppState, router::build_router},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── 1. Environment & tracing ──────────────────────────────────────────────
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "emakao=debug,tower_http=debug,sqlx=warn".into()),
        )
        .json()
        .init();

    // ── 2. Config + AES key ───────────────────────────────────────────────────
    let cfg = Arc::new(Config::from_env()?);

    // Decode once at startup — panics loudly on misconfiguration before any I/O.
    let enc_key = cfg.enc_key();

    // ── 3. Base AppState ──────────────────────────────────────────────────────
    // Builds DB pools, Redis, JWT, SMTP, SMS, M-Pesa, S3, OpenFGA, notification
    // workers, subscription use-cases, and the MiniJinja environment.
    let state = AppState::build(Arc::clone(&cfg)).await?;

    // Cloning PgPool is cheap — it shares the internal connection pool via Arc.
    let platform_pool = state.infra.tenant_pools.platform_pool();

    // ── 4. Customisation layer components ─────────────────────────────────────

    // 4a. Settings cache (DashMap + Redis pub/sub invalidation)
    let settings_cache = Arc::new(SettingsCache::new());

    // 4b. Entitlement cache (plan features merged with per-agency overrides)
    let entitlements = Arc::new(EntitlementCache::new(platform_pool.clone()));

    // 4c. Permission cache (custom_roles → per-user HashSet<String>)
    let permissions = Arc::new(PermissionCache::new(platform_pool.clone()));

    // 4d. Audit logger (fire-and-forget INSERT to partitioned audit_log)
    let audit = Arc::new(AuditLogger::new(platform_pool.clone()));

    // 4e. Provider registry — platform default SMS = Africa's Talking
    let default_sms = Arc::new(AfricasTalkingProvider::from_creds(serde_json::json!({
        "api_key":   cfg.at_api_key,
        "username":  cfg.at_username,
        "sender_id": cfg.at_sender_id,
    }))?);
    let providers = Arc::new(ProviderRegistry::new(default_sms));

    // Load all existing agency integrations into the registry at startup.
    // Errors are logged but do not abort startup — agencies with bad credentials
    // will simply fall back to the platform default provider.
    {
        let ids: Vec<uuid::Uuid> =
            sqlx::query_scalar!("SELECT id FROM agencies WHERE status = 'active'")
                .fetch_all(&platform_pool)
                .await
                .unwrap_or_default();

        for agency_id in ids {
            if let Err(e) = providers
                .load_agency(agency_id, &platform_pool, &enc_key)
                .await
            {
                tracing::warn!(
                    agency = %agency_id,
                    error  = %e,
                    "startup: could not load provider integrations — using platform defaults"
                );
            }
        }
    }

    // 4f. Notification dispatcher (shares the existing MiniJinja env)
    let notifications_dispatcher = Arc::new(NotificationDispatcher {
        pool: platform_pool.clone(),
        providers: Arc::clone(&providers),
        jinja: state.jinja.as_ref().clone(),
    });

    // 4g. Apalis Redis storage for workflow and archival jobs.
    //     We open a dedicated RedisClient (not the pool) so the Apalis queues
    //     can hold their own connection independently of the application pool.
    let apalis_redis = {
        let client = fred::clients::RedisClient::new(
            fred::types::RedisConfig::from_url(&cfg.redis_url)?,
            None,
            None,
            None,
        );
        client.connect();
        client.wait_for_connect().await?;
        client
    };

    // Two separate storage handles — each takes ownership when handed to a worker.
    let workflow_storage: RedisStorage<WorkflowJob> =
        RedisStorage::new(apalis_redis.clone()).await?;
    let archival_storage: RedisStorage<ArchivalJob> =
        RedisStorage::new(apalis_redis.clone()).await?;

    // WorkflowEngine gets its own clone of the storage handle for enqueuing.
    let workflow_engine = Arc::new(WorkflowEngine::new(
        platform_pool.clone(),
        workflow_storage.clone(),
    ));

    // ── 5. Attach customisation layer to AppState ─────────────────────────────
    let state = state.with_customisation(
        Arc::clone(&settings_cache),
        Arc::clone(&entitlements),
        Arc::clone(&permissions),
        Arc::clone(&providers),
        Arc::clone(&audit),
        Arc::clone(&notifications_dispatcher),
        Arc::clone(&workflow_engine),
        enc_key,
    );

    // ── 6. Redis pub/sub invalidation listener ────────────────────────────────
    // Subscribes to "cache:invalidate:agency" and drops local DashMap entries
    // when another pod broadcasts an invalidation.
    {
        let subscriber = fred::clients::SubscriberClient::new(
            fred::types::RedisConfig::from_url(&cfg.redis_url)?,
            None,
            None,
            None,
        );
        subscriber.connect();
        subscriber.wait_for_connect().await?;

        tokio::spawn(run_invalidation_listener(
            subscriber,
            Arc::clone(&settings_cache),
        ));
    }

    // ── 7. Workflow job worker ─────────────────────────────────────────────────
    {
        let state_arc = Arc::new(state.clone());
        let worker = WorkerBuilder::new("workflow-worker")
            .concurrency(cfg.workflow_worker_concurrency)
            .data(state_arc)
            .backend(workflow_storage)
            .build_fn(execute_workflow_job);

        tokio::spawn(async move {
            if let Err(e) = worker.run().await {
                tracing::error!(err = %e, "workflow worker crashed");
            }
        });
    }

    // ── 8. Archival job worker ────────────────────────────────────────────────
    {
        let state_arc = Arc::new(state.clone());
        let worker = WorkerBuilder::new("archival-worker")
            .concurrency(cfg.archival_worker_concurrency)
            .data(state_arc)
            .backend(archival_storage)
            .build_fn(process_archival);

        tokio::spawn(async move {
            if let Err(e) = worker.run().await {
                tracing::error!(err = %e, "archival worker crashed");
            }
        });
    }

    // ── 9. Billing monitor (unchanged) ────────────────────────────────────────
    let billing_monitor = build_billing_monitor(
        BillingContext {
            pool_manager: Arc::clone(&state.infra.tenant_pools),
            notifications: *state.notifications.clone(),
        },
        cfg.redis_url.clone(),
    )
    .await?;

    tokio::spawn(async move {
        if let Err(e) = billing_monitor.run().await {
            tracing::error!(err = %e, "billing monitor crashed");
        }
    });

    // ── 10. HTTP server ───────────────────────────────────────────────────────
    let addr = format!("0.0.0.0:{}", cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let router = build_router(state);

    tracing::info!(addr = %addr, "emakao listening");
    axum::serve(listener, router).await?;

    Ok(())
}
