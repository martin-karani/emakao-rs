use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub rust_log: String,

    pub platform_database_url: String,
    pub tenant_database_url: String,
    pub db_max_connections: u32,
    pub db_min_connections: u32,

    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_expiry_seconds: u64,
    pub jwt_refresh_expiry_seconds: u64,

    pub openfga_url: String,
    pub admin_api_key: String,

    pub mpesa_consumer_key: String,
    pub mpesa_consumer_secret: String,
    pub mpesa_shortcode: String,
    pub mpesa_passkey: String,
    pub mpesa_callback_url: String,
    pub mpesa_base_url: String,

    pub at_api_key: String,
    pub at_username: String,
    pub at_sender_id: Option<String>,

    pub aws_access_key_id: String,
    pub aws_secret_access_key: String,
    pub aws_endpoint_url: Option<String>,
    pub aws_region: String,
    pub s3_bucket: String,

    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_from: String,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,

    pub notification_worker_concurrency: Option<usize>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            port: env_parse("PORT", 3000),
            rust_log: std::env::var("RUST_LOG").unwrap_or_else(|_| "emakao=debug".into()),

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

            admin_api_key: std::env::var("ADMIN_API_KEY").unwrap_or_default(),

            mpesa_consumer_key: std::env::var("MPESA_CONSUMER_KEY").unwrap_or_default(),
            mpesa_consumer_secret: std::env::var("MPESA_CONSUMER_SECRET").unwrap_or_default(),
            mpesa_shortcode: std::env::var("MPESA_SHORTCODE").unwrap_or_default(),
            mpesa_passkey: std::env::var("MPESA_PASSKEY").unwrap_or_default(),
            mpesa_callback_url: std::env::var("MPESA_CALLBACK_URL").unwrap_or_default(),
            mpesa_base_url: std::env::var("MPESA_BASE_URL")
                .unwrap_or_else(|_| "https://sandbox.safaricom.co.ke".into()),

            at_api_key: std::env::var("AT_API_KEY").unwrap_or_default(),
            at_username: std::env::var("AT_USERNAME").unwrap_or_else(|_| "sandbox".into()),
            at_sender_id: std::env::var("AT_SENDER_ID").ok(),

            aws_access_key_id: std::env::var("AWS_ACCESS_KEY_ID").unwrap_or_default(),
            aws_secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY").unwrap_or_default(),
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
        })
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
