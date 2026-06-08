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
    domain::agency::AgencySettings,
    infrastructure::{
        audit::AuditLogger,
        auth::jwt::JwtAuth,
        cache::{
            entitlement_cache::{EntitlementCache, FeatureMap},
            permission_cache::PermissionCache,
            redis_cache::RedisCache,
            settings_cache::SettingsCache,
            subscription_cache::SubscriptionCache,
            token_blacklist::TokenBlacklist,
        },
        db::{
            agency_repository_sqlx::PgAgencyRepo, auth_repository_sqlx::PgAuthRepo,
            pool::AgencyPoolManager, subscription_repository_sqlx::PgSubscriptionRepo,
        },
        email::smtp_adapter::SmtpEmail,
        jobs::workflow::WorkflowEngine,
        notifications::{
            build_notification_components, dispatcher::NotificationDispatcher,
            start_notification_workers,
        },
        openfga::openfga_adapter::OpenFgaAdapter,
        payments::mpesa_adapter::MpesaAdapter,
        providers::ProviderRegistry,
        sms::africa_talking_adapter::AfricasTalkingSms,
        storage::s3_adapter::S3Storage,
    },
};

// ── Sub-structs ───────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct RuntimeState {
    pub started_at: time::OffsetDateTime,
    pub started_instant: Instant,
    pub websocket_channels: Arc<DashMap<Uuid, broadcast::Sender<String>>>,
}

#[derive(Clone)]
pub struct InfraState {
    pub tenant_pools: Arc<AgencyPoolManager>,
    pub redis_cache: Arc<RedisCache>,
    pub redis: RedisPool,
}

#[derive(Clone)]
pub struct IdentityState {
    pub auth_port: Arc<dyn AuthPort>,
    pub auth_repo: Arc<dyn AuthRepository>,
    pub agency_repo: Arc<dyn AgencyRepository>,
    pub auth_uc: Arc<AuthUseCases>,
}

#[derive(Clone)]
pub struct IntegrationState {
    pub email: Arc<dyn EmailPort>,
    pub sms: Arc<dyn SmsPort>,
    pub mpesa: Arc<MpesaAdapter>,
}

// ── Customisation ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct CustomisationState {
    pub settings: Arc<SettingsCache>,
    pub entitlements: Arc<EntitlementCache>,
    pub permissions: Arc<PermissionCache>,
    pub providers: Arc<ProviderRegistry>,
    pub audit: Arc<AuditLogger>,
    pub notifications: Arc<NotificationDispatcher>,
    pub workflow: Arc<WorkflowEngine>,
    pub enc_key: Arc<[u8; 32]>,
}

pub struct AgencyContext {
    pub agency_id: Uuid,
    pub settings: Arc<AgencySettings>,
    pub entitlements: FeatureMap,
}

impl CustomisationState {
    pub async fn agency_ctx(
        &self,
        agency_id: Uuid,
        pool: &sqlx::PgPool,
    ) -> anyhow::Result<AgencyContext> {
        let settings = self.settings.get_or_load(agency_id, pool).await?;
        let entitlements = self.entitlements.resolve(agency_id).await?;
        Ok(AgencyContext {
            agency_id,
            settings,
            entitlements,
        })
    }
}

// ── Use-case bundles ──────────────────────────────────────────────────────────

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

#[derive(Clone)]
pub struct AgencyUseCases {
    pub provision: Arc<ProvisionAgencyUseCase>,
}

// ── AppState ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<crate::config::Config>,
    pub runtime: RuntimeState,
    pub infra: InfraState,
    pub identity: IdentityState,
    pub integration: IntegrationState,
    pub subscription: SubscriptionUseCases,
    pub agency_uc: AgencyUseCases,

    /// `None` until `with_customisation()` is called in `main.rs`.
    pub custom: Option<CustomisationState>,

    // Promoted to top level for middleware fast-path.
    pub jwt: Arc<dyn AuthPort>,
    pub token_blacklist: Arc<TokenBlacklist>,
    pub storage: Arc<dyn StoragePort>,
    // NotificationService holds Arc-backed RedisStorage handles internally so
    // it is already cheap to clone. No outer Arc wrapper needed.
    pub notifications: NotificationService,
    pub openfga: Arc<dyn OpenFgaPort>,
    pub jinja: Arc<minijinja::Environment<'static>>,
}

// ── Customisation builder ─────────────────────────────────────────────────────

impl AppState {
    pub fn with_customisation(
        self,
        settings: Arc<SettingsCache>,
        entitlements: Arc<EntitlementCache>,
        permissions: Arc<PermissionCache>,
        providers: Arc<ProviderRegistry>,
        audit: Arc<AuditLogger>,
        notifications: Arc<NotificationDispatcher>,
        workflow: Arc<WorkflowEngine>,
        enc_key: [u8; 32],
    ) -> Self {
        Self {
            custom: Some(CustomisationState {
                settings,
                entitlements,
                permissions,
                providers,
                audit,
                notifications,
                workflow,
                enc_key: Arc::new(enc_key),
            }),
            ..self
        }
    }

    /// # Panics
    /// Panics if `with_customisation()` has not been called before the server starts.
    pub fn customisation(&self) -> &CustomisationState {
        self.custom
            .as_ref()
            .expect("CustomisationState not initialised — call with_customisation() before serving")
    }

    pub async fn agency_ctx(&self, agency_id: Uuid) -> anyhow::Result<AgencyContext> {
        self.customisation()
            .agency_ctx(agency_id, self.infra.tenant_pools.platform())
            .await
    }

    pub fn pool(&self) -> sqlx::PgPool {
        self.infra.tenant_pools.platform_pool()
    }
}

// ── AppState::build ───────────────────────────────────────────────────────────

impl AppState {
    pub async fn build(cfg: Arc<Config>) -> Result<Self> {
        // ── Platform DB ───────────────────────────────────────────────────────
        let platform_pool = PgPoolOptions::new()
            .max_connections(cfg.db_max_connections)
            .min_connections(cfg.db_min_connections)
            .connect(&cfg.platform_database_url)
            .await
            .context("failed to connect to platform database")?;

        sqlx::migrate!("migrations/platform")
            .run(&platform_pool)
            .await
            .context("platform migration failed")?;

        let pool_manager = Arc::new(AgencyPoolManager::new(
            platform_pool.clone(),
            cfg.tenant_database_url.clone(),
        ));

        // ── Redis ─────────────────────────────────────────────────────────────
        let redis_config =
            fred::types::RedisConfig::from_url(&cfg.redis_url).context("invalid REDIS_URL")?;
        let redis_pool = fred::clients::RedisPool::new(redis_config, None, None, None, 6)
            .context("failed to create Redis pool")?;
        redis_pool.connect();
        redis_pool
            .wait_for_connect()
            .await
            .context("failed to connect to Redis")?;

        let redis_cache = Arc::new(RedisCache::new(redis_pool.clone()));
        let token_blacklist = Arc::new(TokenBlacklist::new(redis_pool.clone()));

        // ── JWT ───────────────────────────────────────────────────────────────
        let jwt = Arc::new(JwtAuth {
            secret: cfg.jwt_secret.clone(),
            expiry_seconds: cfg.jwt_expiry_seconds,
        });

        // ── Repositories ──────────────────────────────────────────────────────
        let auth_repo: Arc<dyn AuthRepository> = Arc::new(PgAuthRepo::new(platform_pool.clone()));
        let agency_repo: Arc<dyn AgencyRepository> =
            Arc::new(PgAgencyRepo::new(platform_pool.clone()));
        let subscription_repo: Arc<dyn SubscriptionRepository> = Arc::new(PgSubscriptionRepo::new(
            platform_pool.clone(),
            pool_manager.clone(),
        ));

        // ── External services ─────────────────────────────────────────────────
        let email: Arc<dyn EmailPort> = Arc::new(
            SmtpEmail::new(
                &cfg.smtp_host,
                cfg.smtp_port,
                cfg.smtp_from.clone(),
                cfg.smtp_username.as_deref(),
                cfg.smtp_password.as_deref(),
            )
            .context("failed to build SMTP transport")?,
        );

        let sms: Arc<dyn SmsPort> = Arc::new(AfricasTalkingSms::new(
            cfg.at_api_key.clone().unwrap_or_default(),
            cfg.at_username.clone(),
            cfg.at_sender_id.clone(),
        ));

        let mpesa = Arc::new(MpesaAdapter::new(
            cfg.mpesa_consumer_key.clone().unwrap_or_default(),
            cfg.mpesa_consumer_secret.clone().unwrap_or_default(),
            cfg.mpesa_shortcode.clone().unwrap_or_default(),
            cfg.mpesa_passkey.clone().unwrap_or_default(),
            cfg.mpesa_callback_url.clone(),
            cfg.mpesa_base_url.clone(),
        ));

        let storage: Arc<dyn StoragePort> = Arc::new(
            S3Storage::new(
                cfg.aws_access_key_id.as_deref().unwrap_or_default(),
                cfg.aws_secret_access_key.as_deref().unwrap_or_default(),
                &cfg.aws_region,
                cfg.s3_bucket.clone(),
                cfg.aws_endpoint_url.as_deref(),
            )
            .await,
        );

        // ── OpenFGA ───────────────────────────────────────────────────────────
        let openfga: Arc<dyn OpenFgaPort> = Arc::new(OpenFgaAdapter::new(cfg.openfga_url.clone()));

        // ── Subscription use-cases ────────────────────────────────────────────
        let sub_cache = Arc::new(SubscriptionCache::new(redis_pool.clone()));

        let (events_tx, _) = tokio::sync::broadcast::channel(100);

        let subscription = SubscriptionUseCases {
            get_state: Arc::new(GetSubscriptionStateUseCase::new(
                Arc::clone(&subscription_repo),
                Arc::clone(&sub_cache),
            )),
            get_entitlements: Arc::new(GetEntitlementsUseCase::new(
                Arc::clone(&subscription_repo),
                Arc::clone(&sub_cache),
            )),
            list_plans: Arc::new(ListPlansUseCase::new(Arc::clone(&subscription_repo))),
            get_usage: Arc::new(GetUsageUseCase::new(
                Arc::clone(&subscription_repo),
                Arc::clone(&sub_cache),
            )),
            change_plan: Arc::new(ChangePlanUseCase {
                repo: Arc::clone(&subscription_repo),
                cache: Arc::clone(&sub_cache),
                events: events_tx.clone(),
            }),
            cancel_subscription: Arc::new(CancelSubscriptionUseCase {
                repo: Arc::clone(&subscription_repo),
                cache: Arc::clone(&sub_cache),
                events: events_tx.clone(),
            }),
            set_override: Arc::new(SetFeatureOverrideUseCase {
                repo: Arc::clone(&subscription_repo),
                cache: Arc::clone(&sub_cache),
            }),
            remove_override: Arc::new(RemoveFeatureOverrideUseCase {
                repo: Arc::clone(&subscription_repo),
                cache: Arc::clone(&sub_cache),
            }),
            list_invoices: Arc::new(ListInvoicesUseCase {
                repo: Arc::clone(&subscription_repo),
            }),
            record_payment: Arc::new(RecordPaymentUseCase {
                repo: Arc::clone(&subscription_repo),
                cache: Arc::clone(&sub_cache),
            }),
            initiate_payment: Arc::new(InitiateSubscriptionPaymentUseCase {
                repo: Arc::clone(&subscription_repo),
                mpesa: Arc::clone(&mpesa),
            }),
            repo: Arc::clone(&subscription_repo),
            cache: Arc::clone(&sub_cache),
        };

        // ── Auth use-cases ────────────────────────────────────────────────────
        let auth_uc = Arc::new(AuthUseCases {
            refresh: Arc::new(RefreshTokenUseCase::new(
                Arc::clone(&jwt) as Arc<dyn AuthPort>
            )),
        });

        // ── Agency use-cases ──────────────────────────────────────────────────
        let agency_uc = AgencyUseCases {
            provision: Arc::new(ProvisionAgencyUseCase::new(
                Arc::clone(&agency_repo),
                Arc::clone(&pool_manager),
                Arc::clone(&openfga),
            )),
        };

        // ── MiniJinja ─────────────────────────────────────────────────────────
        let mut jinja_env = minijinja::Environment::new();
        jinja_env.set_loader(minijinja::path_loader("templates"));
        jinja_env.set_undefined_behavior(minijinja::UndefinedBehavior::Lenient);
        jinja_env.add_global("current_year", time::OffsetDateTime::now_utc().year());
        let jinja = Arc::new(jinja_env);

        // ── Notification workers ──────────────────────────────────────────────
        let components = build_notification_components("templates", &cfg.redis_url).await?;

        let notifications = components.service.clone();

        // start_notification_workers moves the storage handles out of `components`
        // and spawns the email + SMS Apalis workers.
        start_notification_workers(
            components,
            Arc::clone(&email),
            Arc::clone(&sms),
            cfg.notification_worker_concurrency.unwrap_or(4),
        );

        Ok(Self {
            config: Arc::clone(&cfg),
            runtime: RuntimeState {
                started_at: time::OffsetDateTime::now_utc(),
                started_instant: Instant::now(),
                websocket_channels: Arc::new(DashMap::new()),
            },
            infra: InfraState {
                tenant_pools: pool_manager,
                redis_cache,
                redis: redis_pool,
            },
            identity: IdentityState {
                auth_port: Arc::clone(&jwt) as Arc<dyn AuthPort>,
                auth_repo,
                agency_repo,
                auth_uc,
            },
            integration: IntegrationState { email, sms, mpesa },
            subscription,
            agency_uc,
            custom: None,
            jwt: Arc::clone(&jwt) as Arc<dyn AuthPort>,
            token_blacklist,
            storage,
            notifications,
            openfga,
            jinja,
        })
    }
}
