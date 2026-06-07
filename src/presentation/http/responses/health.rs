// ── REFACTORED ────────────────────────────────────────────────────────────────
// Changes from original:
//   - Added rfc3339 serialization to OffsetDateTime fields
//   - Applied #[serde(skip_serializing_if)] to optional fields
// ─────────────────────────────────────────────────────────────────────────────

use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct ComponentStatus {
    pub ok: bool,
    pub critical: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct DependencyStatus {
    pub database: ComponentStatus,
    pub redis: ComponentStatus,
    pub openfga: ComponentStatus,
}

#[derive(Serialize, ToSchema)]
pub struct DatabasePoolStatus {
    pub size: u32,
    pub idle: usize,
    pub closed: bool,
    pub cached_tenant_pools: usize,
}

#[derive(Serialize, ToSchema)]
pub struct RuntimeStatus {
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub timestamp: OffsetDateTime,
    pub uptime_seconds: i64,
    pub websocket_channels: usize,
}

#[derive(Serialize, ToSchema)]
pub struct ServiceInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub port: u16,
}

#[derive(Serialize, ToSchema)]
pub struct ConfigurationStatus {
    pub admin_api_key_configured: bool,
    pub smtp_configured: bool,
    pub mpesa_configured: bool,
    pub africa_talking_configured: bool,
    pub aws_s3_configured: bool,
}

#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: ServiceInfo,
    pub runtime: RuntimeStatus,
    pub dependencies: DependencyStatus,
    pub database_pool: DatabasePoolStatus,
    pub configuration: ConfigurationStatus,
}

#[derive(Serialize, ToSchema)]
pub struct ReadyResponse {
    pub status: &'static str,
    pub ready: bool,
    pub service: ServiceInfo,
    pub runtime: RuntimeStatus,
    pub dependencies: DependencyStatus,
    pub database_pool: DatabasePoolStatus,
    pub configuration: ConfigurationStatus,
}
