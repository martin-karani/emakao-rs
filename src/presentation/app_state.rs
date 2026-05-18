//
// Application state — the composition root for the Axum server.
//
// ## Structure
//
// Rather than one flat bag of every adapter and use-case, `AppState` groups
// its fields into four focused sub-structs:
//
// | Sub-struct           | Responsibility                                    |
// |----------------------|---------------------------------------------------|
// | `RuntimeState`       | Observability — start time, uptime, WS channels   |
// | `InfraState`         | Data layer — DB pool manager, Redis cache & pool   |
// | `IdentityState`      | Auth — JWT port, user/invite repo, auth use-cases  |
// | `IntegrationState`   | External services — email, SMS, M-Pesa             |
//
// Two fields are also promoted **flat** onto `AppState` itself so that Axum
// middleware (`require_auth`) can access them without traversing sub-structs:
//
// * `jwt`               — the `AuthPort` used to decode Bearer tokens
// * `token_blacklist`   — the Redis-backed revocation list (logout / refresh)
//
// ## Storage
//
// `AppState` carries a single `Arc<dyn StoragePort>` built once at startup.
// All handlers (upload, documents, payments, …) must use `state.storage` — do
// NOT call `S3Storage::from_config` or `S3Storage::new` from within a handler.
// Building a new S3 client per request wastes connection-pool initialisation
// and blocks the async executor if done synchronously.

use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use dashmap::DashMap;
use fred::clients::RedisPool;
use fred::interfaces::ClientLike;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::{
    application::{
        notifications::NotificationService,
        ports::{
            agency_repository::AgencyRepository, auth_port::AuthPort,
            auth_repository::AuthRepository, email_port::EmailPort, openfga_port::OpenFgaPort,
            sms_port::SmsPort, storage_port::StoragePort,
            subscription_repository::SubscriptionRepository,
        },
        use_cases::{
            agency::provision::ProvisionAgencyUseCase,
            auth::refresh_token::RefreshTokenUseCase,
            subscription::{
                cancel_subscription::CancelSubscriptionUseCase, change_plan::ChangePlanUseCase,
                get_entitlements::GetEntitlementsUseCase, get_state::GetSubscriptionStateUseCase,
                get_usage::GetUsageUseCase,
                initiate_subscription_payment::InitiateSubscriptionPaymentUseCase,
                list_invoices::ListInvoicesUseCase, list_plans::ListPlansUseCase,
                record_payment::RecordPaymentUseCase,
                remove_feature_override::RemoveFeatureOverrideUseCase,
                set_feature_override::SetFeatureOverrideUseCase,
            },
        },
    },
    config::Config,
    domain::subscription::SubscriptionEvent,
    infrastructure::{
        auth::jwt::JwtAuth,
        cache::{
            redis_cache::RedisCache, subscription_cache::SubscriptionCache,
            token_blacklist::TokenBlacklist,
        },
        db::{
            agency_repository_sqlx::PgAgencyRepo, auth_repository_sqlx::PgAuthRepo,
            pool::AgencyPoolManager, subscription_repository_sqlx::PgSubscriptionRepo,
        },
        email::smtp_adapter::SmtpEmail,
        notifications::{build_notification_components, start_notification_workers},
        openfga::openfga_adapter::OpenFgaAdapter,
        payments::mpesa_adapter::MpesaAdapter,
        sms::africa_talking_adapter::AfricasTalkingSms,
        storage::s3_adapter::S3Storage,
    },
};

// ── Sub-structs ───────────────────────────────────────────────────────────────

/// Observability / runtime metadata.
#[derive(Clone)]
pub struct RuntimeState {
    pub started_at: time::OffsetDateTime,
    pub started_instant: Instant,
    pub websocket_channels: Arc<DashMap<Uuid, broadcast::Sender<String>>>,
}

/// Database and cache layer.
///
/// `redis` is exposed here so that out-of-process workers (billing monitor)
/// can obtain a pool handle without going through `AppState`.
#[derive(Clone)]
pub struct InfraState {
    pub tenant_pools: Arc<AgencyPoolManager>,
    pub redis_cache: Arc<RedisCache>,
    /// Raw Fred pool — share with billing workers and any component that needs
    /// direct pub/sub or raw Redis commands.
    pub redis: RedisPool,
}

/// Authentication — JWT verification, user/invite repository, auth use-cases.
#[derive(Clone)]
pub struct IdentityState {
    pub auth_port: Arc<dyn AuthPort>,
    pub auth_repo: Arc<dyn AuthRepository>,
    /// Stateless use-cases (refresh token, etc.).
    pub auth_uc: Arc<AuthUseCases>,
}

/// External service adapters.
#[derive(Clone)]
pub struct IntegrationState {
    pub email: Arc<dyn EmailPort>,
    pub sms: Arc<dyn SmsPort>,
    pub mpesa: Arc<MpesaAdapter>,
}

// ── Domain use-case bundles (kept flat on AppState) ───────────────────────────

#[derive(Clone)]
pub struct AuthUseCases {
    pub refresh: Arc<RefreshTokenUseCase>,
}

#[derive(Clone)]
pub struct SubscriptionUseCases {
    pub get_state: Arc<GetSubscriptionStateUseCase>,
    pub get_entitlements: Arc<GetEntitlementsUseCase>,
    pub list_plans: Arc<ListPlansUseCase>,
    pub get_usage: Arc<GetUsageUseCase>,
    pub change_plan: Arc<ChangePlanUseCase>,
    pub cancel_subscription: Arc<CancelSubscriptionUseCase>,
    pub set_override: Arc<SetFeatureOverrideUseCase>,
    pub remove_override: Arc<RemoveFeatureOverrideUseCase>,
    pub list_invoices: Arc<ListInvoicesUseCase>,
    pub record_payment: Arc<RecordPaymentUseCase>,
    pub initiate_payment: Arc<InitiateSubscriptionPaymentUseCase>,
    pub repo: Arc<dyn SubscriptionRepository>,
    pub cache: Arc<SubscriptionCache>,
}

/// Agency management use-cases (admin only).
#[derive(Clone)]
pub struct AgencyUseCases {
    pub provision: Arc<ProvisionAgencyUseCase>,
}

// ── Top-level state ───────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,

    // ── Sub-structs ───────────────────────────────────────────────────────────
    /// Observability, start time, WS channels.
    pub runtime: Arc<RuntimeState>,
    /// DB pool manager + Redis cache + raw Redis pool.
    pub infra: Arc<InfraState>,
    /// Auth JWT port, auth repo, auth use-cases.
    pub identity: Arc<IdentityState>,
    /// Subscription use-cases (accessed by middleware & many handlers).
    pub subscription: Arc<SubscriptionUseCases>,
    /// Agency provisioning use-cases (admin only).
    pub agency: Arc<AgencyUseCases>,
    /// Email, SMS, M-Pesa adapters.
    pub integrations: Arc<IntegrationState>,
    /// Async notification queue service.
    pub notifications: NotificationService,
    /// OpenFGA authorisation port.
    pub openfga: Arc<dyn OpenFgaPort>,

    /// Shared S3 storage adapter — built once at startup, reused by every
    /// handler. Use `state.storage` everywhere; never call `S3Storage::new`
    /// or `S3Storage::from_config` from a handler.
    pub storage: Arc<dyn StoragePort>,

    // ── Flat middleware shortcuts ─────────────────────────────────────────────
    /// JWT decode/verify — used directly by `require_auth` middleware.
    /// Same underlying adapter as `identity.auth_port`.
    pub jwt: Arc<dyn AuthPort>,
    /// Token revocation list — used by `require_auth` to check logout/refresh.
    pub token_blacklist: Arc<TokenBlacklist>,
}

impl AppState {
    pub async fn build(cfg: Arc<Config>) -> Result<Self> {
        // ── Platform DB ───────────────────────────────────────────────────────
        let platform_pool = PgPoolOptions::new()
            .max_connections(cfg.db_max_connections)
            .min_connections(cfg.db_min_connections)
            .connect(&cfg.platform_database_url)
            .await
            .context("failed to connect to platform postgres")?;

        let agency_pools = Arc::new(AgencyPoolManager::new(
            platform_pool.clone(),
            cfg.tenant_database_url.clone(),
        ));

        // ── Redis ─────────────────────────────────────────────────────────────
        let redis = RedisPool::new(
            fred::types::RedisConfig::from_url(&cfg.redis_url).context("invalid REDIS_URL")?,
            None,
            None,
            None,
            8,
        )
        .context("redis pool init failed")?;
        redis.connect();
        redis
            .wait_for_connect()
            .await
            .context("redis connect timeout")?;

        let sub_cache = Arc::new(SubscriptionCache::new(redis.clone()));
        let redis_cache = Arc::new(RedisCache::new(redis.clone()));
        let token_blacklist = Arc::new(TokenBlacklist::new(redis.clone()));

        // ── InfraState ────────────────────────────────────────────────────────
        let infra = Arc::new(InfraState {
            tenant_pools: Arc::clone(&agency_pools),
            redis_cache,
            redis: redis.clone(),
        });

        // ── IdentityState ─────────────────────────────────────────────────────
        let auth_port: Arc<dyn AuthPort> = Arc::new(JwtAuth {
            secret: cfg.jwt_secret.clone(),
            expiry_seconds: cfg.jwt_expiry_seconds,
        });
        let auth_repo: Arc<dyn AuthRepository> = Arc::new(PgAuthRepo::new(platform_pool.clone()));

        let auth_uc = Arc::new(AuthUseCases {
            refresh: Arc::new(RefreshTokenUseCase::new(Arc::clone(&auth_port))),
        });

        let identity = Arc::new(IdentityState {
            auth_port: Arc::clone(&auth_port),
            auth_repo,
            auth_uc,
        });

        // ── Storage (S3 / MinIO) ──────────────────────────────────────────────
        // Built once here; every handler that touches files uses state.storage.
        // This avoids the costly per-request client initialisation that the old
        // `build_storage()` / `S3Storage::from_config()` pattern caused.
        let storage: Arc<dyn StoragePort> = Arc::new(
            S3Storage::new(
                &cfg.aws_access_key_id,
                &cfg.aws_secret_access_key,
                &cfg.aws_region,
                cfg.s3_bucket.clone(),
                cfg.aws_endpoint_url.as_deref(),
            )
            .await,
        );

        // ── IntegrationState ──────────────────────────────────────────────────
        let email: Arc<dyn EmailPort> = Arc::new(
            SmtpEmail::new(
                &cfg.smtp_host,
                cfg.smtp_port,
                cfg.smtp_from.clone(),
                cfg.smtp_username.as_deref(),
                cfg.smtp_password.as_deref(),
            )
            .context("SMTP adapter init failed")?,
        );

        let sms: Arc<dyn SmsPort> = Arc::new(AfricasTalkingSms::new(
            cfg.at_api_key.clone(),
            cfg.at_username.clone(),
            cfg.at_sender_id.clone(),
        ));

        let mpesa = Arc::new(MpesaAdapter::new(
            cfg.mpesa_consumer_key.clone(),
            cfg.mpesa_consumer_secret.clone(),
            cfg.mpesa_shortcode.clone(),
            cfg.mpesa_passkey.clone(),
            cfg.mpesa_callback_url.clone(),
            cfg.mpesa_base_url.clone(),
        ));

        let integrations = Arc::new(IntegrationState {
            email: Arc::clone(&email),
            sms: Arc::clone(&sms),
            mpesa,
        });

        // ── Notifications ─────────────────────────────────────────────────────
        let templates_dir = std::env::current_dir()
            .context("cannot determine cwd")?
            .join("templates");

        let notification_components = build_notification_components(
            templates_dir
                .to_str()
                .context("templates dir is not valid UTF-8")?,
            &cfg.redis_url,
        )
        .await
        .context("notification service init failed")?;

        let notifications = notification_components.service.clone();

        start_notification_workers(
            notification_components,
            Arc::clone(&email),
            Arc::clone(&sms),
            cfg.notification_worker_concurrency.unwrap_or(4),
        );

        // ── OpenFGA ───────────────────────────────────────────────────────────
        let openfga_adapter = Arc::new(OpenFgaAdapter::new(cfg.openfga_url.clone()));
        let openfga: Arc<dyn OpenFgaPort> = Arc::clone(&openfga_adapter) as Arc<dyn OpenFgaPort>;

        // ── Agency use-cases ──────────────────────────────────────────────────
        let agency_repo: Arc<dyn AgencyRepository> =
            Arc::new(PgAgencyRepo::new(platform_pool.clone()));

        let default_model: serde_json::Value =
            serde_json::from_str(include_str!("../../resources/fga/default_model.json"))
                .context("resources/fga/default_model.json is not valid JSON")?;

        let agency = Arc::new(AgencyUseCases {
            provision: Arc::new(ProvisionAgencyUseCase {
                agency_repo: Arc::clone(&agency_repo),
                openfga: Arc::clone(&openfga),
                pool_manager: Arc::clone(&agency_pools),
                default_model,
            }),
        });

        // ── Subscription use-cases ────────────────────────────────────────────
        let (event_tx, _event_rx) = tokio::sync::broadcast::channel::<SubscriptionEvent>(256);

        let tenant_pools_clone = Arc::clone(&agency_pools);
        let sub_repo: Arc<dyn SubscriptionRepository> = Arc::new(PgSubscriptionRepo::new(
            platform_pool.clone(),
            move |agency_id| {
                let pools = Arc::clone(&tenant_pools_clone);
                Box::pin(async move { pools.for_agency(agency_id).await })
            },
        ));

        let subscription = Arc::new(SubscriptionUseCases {
            get_state: Arc::new(GetSubscriptionStateUseCase::new(
                sub_repo.clone(),
                sub_cache.clone(),
            )),
            get_entitlements: Arc::new(GetEntitlementsUseCase::new(
                sub_repo.clone(),
                sub_cache.clone(),
            )),
            list_plans: Arc::new(ListPlansUseCase::new(sub_repo.clone())),
            get_usage: Arc::new(GetUsageUseCase::new(sub_repo.clone(), sub_cache.clone())),
            change_plan: Arc::new(ChangePlanUseCase {
                repo: sub_repo.clone(),
                cache: sub_cache.clone(),
                events: event_tx.clone(),
            }),
            cancel_subscription: Arc::new(CancelSubscriptionUseCase {
                repo: sub_repo.clone(),
                cache: sub_cache.clone(),
                events: event_tx.clone(),
            }),
            set_override: Arc::new(SetFeatureOverrideUseCase {
                repo: sub_repo.clone(),
                cache: sub_cache.clone(),
            }),
            remove_override: Arc::new(RemoveFeatureOverrideUseCase {
                repo: sub_repo.clone(),
                cache: sub_cache.clone(),
            }),
            list_invoices: Arc::new(ListInvoicesUseCase {
                repo: sub_repo.clone(),
            }),
            record_payment: Arc::new(RecordPaymentUseCase {
                repo: sub_repo.clone(),
                cache: sub_cache.clone(),
            }),
            initiate_payment: Arc::new(InitiateSubscriptionPaymentUseCase {
                repo: sub_repo.clone(),
                mpesa: Arc::clone(&integrations.mpesa),
            }),
            repo: sub_repo,
            cache: sub_cache,
        });

        // ── RuntimeState ──────────────────────────────────────────────────────
        let runtime = Arc::new(RuntimeState {
            started_at: time::OffsetDateTime::now_utc(),
            started_instant: Instant::now(),
            websocket_channels: Arc::new(DashMap::new()),
        });

        tracing::info!(
            "Notification workers started with concurrency = {}",
            cfg.notification_worker_concurrency.unwrap_or(4)
        );

        tracing::info!("AppState built successfully ✓");

        Ok(Self {
            config: cfg,
            runtime,
            infra,
            identity,
            subscription,
            agency,
            integrations,
            notifications,
            openfga,
            storage,
            // Flat middleware shortcuts — same underlying instances, no extra cost.
            jwt: auth_port,
            token_blacklist,
        })
    }
}
