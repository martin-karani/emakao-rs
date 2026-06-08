use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub rust_log: String,
    pub app_base_url: String,

    pub platform_database_url: String,
    pub tenant_database_url: String,
    pub db_max_connections: u32,
    pub db_min_connections: u32,

    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_expiry_seconds: u64,
    pub jwt_refresh_expiry_seconds: u64,

    pub openfga_url: String,
    pub admin_api_key: Option<String>,

    pub mpesa_consumer_key: Option<String>,
    pub mpesa_consumer_secret: Option<String>,
    pub mpesa_shortcode: Option<String>,
    pub mpesa_passkey: Option<String>,
    pub mpesa_callback_url: String,
    pub mpesa_base_url: String,

    pub at_api_key: Option<String>,
    pub at_username: String,
    pub at_sender_id: Option<String>,

    pub aws_access_key_id: Option<String>,
    pub aws_secret_access_key: Option<String>,
    pub aws_endpoint_url: Option<String>,
    pub aws_region: String,
    pub s3_bucket: String,

    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_from: String,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,

    pub notification_worker_concurrency: Option<usize>,

    // ── NEW: customisation layer ──────────────────────────────────────────────
    /// 64 hex characters (32 bytes) — AES-256-GCM key for
    /// `agency_integrations.credentials`.
    /// Generate: `openssl rand -hex 32`
    pub credentials_enc_key: String,

    /// Apalis concurrency for the workflow job worker (default 4).
    pub workflow_worker_concurrency: usize,

    /// Apalis concurrency for the archival job worker (default 2).
    pub archival_worker_concurrency: usize,

    /// CORS allowed origins (ALLOWED_ORIGINS env var, comma-separated).
    /// Example: "https://app.emakao.co.ke,https://residents.emakao.co.ke"
    /// An empty list means deny-all in production, permissive in debug.
    pub allowed_origins: Vec<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let config = Self {
            port: env_parse("PORT", 3000),
            rust_log: std::env::var("RUST_LOG").unwrap_or_else(|_| "emakao=debug".into()),
            app_base_url: std::env::var("APP_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:3000".into()),

            platform_database_url: required("PLATFORM_DATABASE_URL")
                .context("PLATFORM_DATABASE_URL must be set")?,
            tenant_database_url: required("AGENCY_DATABASE_URL")
                .context("AGENCY_DATABASE_URL must be set")?,
            db_max_connections: env_parse("DB_MAX_CONNECTIONS", 20),
            db_min_connections: env_parse("DB_MIN_CONNECTIONS", 2),

            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".into()),

            jwt_secret: required("JWT_SECRET")?,
            jwt_expiry_seconds: env_parse("JWT_EXPIRY_SECONDS", 86_400),
            jwt_refresh_expiry_seconds: env_parse("JWT_REFRESH_EXPIRY_SECONDS", 2_592_000),

            openfga_url: std::env::var("OPENFGA_URL")
                .unwrap_or_else(|_| "http://localhost:8080".into()),

            admin_api_key: std::env::var("ADMIN_API_KEY").ok(),

            mpesa_consumer_key: std::env::var("MPESA_CONSUMER_KEY").ok(),
            mpesa_consumer_secret: std::env::var("MPESA_CONSUMER_SECRET").ok(),
            mpesa_shortcode: std::env::var("MPESA_SHORTCODE").ok(),
            mpesa_passkey: std::env::var("MPESA_PASSKEY").ok(),
            mpesa_callback_url: std::env::var("MPESA_CALLBACK_URL")
                .unwrap_or_else(|_| "http://localhost:3000/api/v1/webhooks/mpesa".into()),
            mpesa_base_url: std::env::var("MPESA_BASE_URL")
                .unwrap_or_else(|_| "https://sandbox.safaricom.co.ke".into()),

            at_api_key: std::env::var("AT_API_KEY").ok(),
            at_username: std::env::var("AT_USERNAME").unwrap_or_else(|_| "sandbox".into()),
            at_sender_id: std::env::var("AT_SENDER_ID").ok(),

            aws_access_key_id: std::env::var("AWS_ACCESS_KEY_ID").ok(),
            aws_secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY").ok(),
            aws_endpoint_url: std::env::var("AWS_ENDPOINT_URL").ok(),
            aws_region: std::env::var("AWS_REGION").unwrap_or_else(|_| "af-south-1".into()),
            s3_bucket: std::env::var("S3_BUCKET").unwrap_or_else(|_| "emakao".into()),

            smtp_host: std::env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".into()),
            smtp_port: env_parse("SMTP_PORT", 1025),
            smtp_from: std::env::var("SMTP_FROM").unwrap_or_else(|_| "noreply@emakao.co.ke".into()),
            smtp_username: std::env::var("SMTP_USERNAME").ok(),
            smtp_password: std::env::var("SMTP_PASSWORD").ok(),

            notification_worker_concurrency: std::env::var("NOTIFICATION_WORKER_CONCURRENCY")
                .ok()
                .map(|v| v.parse().unwrap_or(4)),

            // ── NEW ──────────────────────────────────────────────────────────
            credentials_enc_key: required("CREDENTIALS_ENC_KEY")
                .context("CREDENTIALS_ENC_KEY must be set for security")?,

            workflow_worker_concurrency: env_parse("WORKFLOW_WORKER_CONCURRENCY", 4),
            archival_worker_concurrency: env_parse("ARCHIVAL_WORKER_CONCURRENCY", 2),

            allowed_origins: std::env::var("ALLOWED_ORIGINS")
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect(),
        };

        config.validate()?;
        Ok(config)
    }

    /// Decode `credentials_enc_key` to a fixed 32-byte array.
    /// Panics at startup if the key is malformed — intentional; bad key = no start.
    pub fn enc_key(&self) -> [u8; 32] {
        crate::infrastructure::crypto::load_enc_key()
            .expect("CREDENTIALS_ENC_KEY must be a valid 64-char hex string")
    }

    /// Validate that critical secrets are present.
    /// In a production environment, we want to fail loudly if keys are missing.
    /// In development, we might allow some to be missing but warn.
    pub fn validate(&self) -> Result<()> {
        let is_prod = std::env::var("APP_ENV")
            .map(|v| v == "production")
            .unwrap_or(false);

        if is_prod {
            if self.mpesa_consumer_key.is_none() {
                anyhow::bail!("MPESA_CONSUMER_KEY is missing in production!");
            }
            if self.at_api_key.is_none() {
                anyhow::bail!("AT_API_KEY is missing in production!");
            }
            if self.aws_access_key_id.is_none() {
                anyhow::bail!("AWS_ACCESS_KEY_ID is missing in production!");
            }
        } else {
            if self.mpesa_consumer_key.is_none() {
                tracing::warn!("MPESA_CONSUMER_KEY is missing (M-Pesa payments will fail)");
            }
            if self.at_api_key.is_none() {
                tracing::warn!("AT_API_KEY is missing (SMS/AT features will fail)");
            }
            if self.aws_access_key_id.is_none() {
                tracing::warn!("AWS_ACCESS_KEY_ID is missing (S3 uploads will fail)");
            }
        }

        Ok(())
    }
}

fn required(key: &str) -> Result<String> {
    std::env::var(key).with_context(|| format!("environment variable `{key}` is required"))
}

fn env_parse<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
