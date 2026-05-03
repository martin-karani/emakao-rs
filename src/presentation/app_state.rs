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
        ports::{
            agency_repository::AgencyRepository, auth_port::AuthPort,
            auth_repository::AuthRepository, email_port::EmailPort, openfga_port::OpenFgaPort,
            sms_port::SmsPort, subscription_repository::SubscriptionRepository,
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
        cache::{redis_cache::RedisCache, subscription_cache::SubscriptionCache},
        db::{
            agency_repository_sqlx::PgAgencyRepo, auth_repository_sqlx::PgAuthRepo,
            pool::AgencyPoolManager, subscription_repository_sqlx::PgSubscriptionRepo,
        },
        email::smtp_adapter::SmtpEmail,
        openfga::openfga_adapter::OpenFgaAdapter,
        payments::mpesa_adapter::MpesaAdapter,
        sms::africa_talking_adapter::AfricasTalkingSms,
    },
};

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

// ── AppState ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub started_at: time::OffsetDateTime,
    pub started_instant: Instant,
    pub tenant_pools: Arc<AgencyPoolManager>,
    pub redis_cache: Arc<RedisCache>,
    pub auth_port: Arc<dyn AuthPort>,
    pub auth_repo: Arc<dyn AuthRepository>,
    pub auth: Arc<AuthUseCases>, // only refresh token
    pub subscription: Arc<SubscriptionUseCases>,
    pub agency: Arc<AgencyUseCases>,
    pub mpesa: Arc<MpesaAdapter>,
    pub email: Arc<dyn EmailPort>,
    pub sms: Arc<dyn SmsPort>,
    pub websocket_channels: Arc<DashMap<Uuid, broadcast::Sender<String>>>,
    pub openfga: Arc<dyn OpenFgaPort>,
}

impl AppState {
    pub async fn build(cfg: Arc<Config>) -> Result<Self> {
        let started_at = time::OffsetDateTime::now_utc();
        let started_instant = Instant::now();

        // ── Platform DB ───────────────────────────────────────────────────────
        let platform_pool = PgPoolOptions::new()
            .max_connections(cfg.db_max_connections)
            .min_connections(cfg.db_min_connections)
            .connect(&cfg.platform_database_url)
            .await
            .context("failed to connect to platform postgres")?;

        // ── Tenant pool manager ───────────────────────────────────────────────
        let tenant_pools = Arc::new(AgencyPoolManager::new(
            platform_pool.clone(),
            cfg.tenant_database_url.clone(),
        ));

        // ── Auth repository (new identity layer) ──────────────────────────────
        let auth_repo: Arc<dyn AuthRepository> = Arc::new(PgAuthRepo::new(platform_pool.clone()));

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

        // ── Adapters ──────────────────────────────────────────────────────────
        let auth_port: Arc<dyn AuthPort> = Arc::new(JwtAuth {
            secret: cfg.jwt_secret.clone(),
            expiry_seconds: cfg.jwt_expiry_seconds,
        });

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

        let openfga_adapter = Arc::new(OpenFgaAdapter::new(cfg.openfga_url.clone()));
        let openfga: Arc<dyn OpenFgaPort> = Arc::clone(&openfga_adapter) as Arc<dyn OpenFgaPort>;

        // ── Platform repositories ─────────────────────────────────────────────
        let agency_repo: Arc<dyn AgencyRepository> =
            Arc::new(PgAgencyRepo::new(platform_pool.clone()));

        // ── Load default OpenFGA authorization model ──────────────────────────
        let default_model: serde_json::Value =
            serde_json::from_str(include_str!("../../resources/fga/default_model.json"))
                .context("resources/fga/default_model.json is not valid JSON")?;

        // ── Agency use-cases ──────────────────────────────────────────────────
        let agency = Arc::new(AgencyUseCases {
            provision: Arc::new(ProvisionAgencyUseCase {
                agency_repo: Arc::clone(&agency_repo),
                openfga: Arc::clone(&openfga),
                pool_manager: Arc::clone(&tenant_pools),
                default_model,
            }),
        });

        // ── Subscription repo ─────────────────────────────────────────────────
        let (event_tx, _event_rx) = tokio::sync::broadcast::channel::<SubscriptionEvent>(256);

        let tenant_pools_clone = Arc::clone(&tenant_pools);
        let sub_repo: Arc<dyn SubscriptionRepository> = Arc::new(PgSubscriptionRepo::new(
            platform_pool.clone(),
            move |agency_id| {
                let pools = Arc::clone(&tenant_pools_clone);
                Box::pin(async move { pools.for_agency(agency_id).await })
            },
        ));

        // ── Auth use-cases (only refresh token remains) ───────────────────────
        let auth = Arc::new(AuthUseCases {
            refresh: Arc::new(RefreshTokenUseCase::new(Arc::clone(&auth_port))),
        });

        // ── Subscription use-cases ────────────────────────────────────────────
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
                mpesa: Arc::clone(&mpesa),
            }),
            repo: sub_repo,
            cache: sub_cache,
        });

        tracing::info!("AppState built successfully ✓");

        Ok(Self {
            config: cfg,
            started_at,
            started_instant,
            tenant_pools,
            redis_cache,
            auth_port,
            auth_repo,
            auth,
            subscription,
            agency,
            mpesa,
            email,
            sms,
            websocket_channels: Arc::new(DashMap::new()),
            openfga,
        })
    }
}
