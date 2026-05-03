use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;

use crate::presentation::app_state::AppState;

#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub timestamp: OffsetDateTime,
}

#[derive(Serialize, ToSchema)]
pub struct ReadyResponse {
    pub status: &'static str,
    pub database: ComponentStatus,
    pub timestamp: OffsetDateTime,
}

#[derive(Serialize, ToSchema)]
pub struct ComponentStatus {
    pub ok: bool,
    pub latency_ms: Option<u128>,
}

/// Liveness probe — always 200 while the process is alive
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Process is alive", body = HealthResponse),
    ),
    tag = "Health"
)]
pub async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        timestamp: OffsetDateTime::now_utc(),
    })
}

#[utoipa::path(
    get,
    path = "/ready",
    responses(
        (status = 200, description = "Service is ready",         body = ReadyResponse),
        (status = 503, description = "Service is not ready yet", body = ReadyResponse),
    ),
    tag = "Health"
)]
pub async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    let start = std::time::Instant::now();

    let db_ok = sqlx::query("SELECT 1")
        .execute(state.tenant_pools.platform())
        .await
        .is_ok();

    let latency_ms = start.elapsed().as_millis();

    let status_code = if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status_code,
        Json(ReadyResponse {
            status: if db_ok { "ready" } else { "not_ready" },
            database: ComponentStatus {
                ok: db_ok,
                latency_ms: Some(latency_ms),
            },
            timestamp: OffsetDateTime::now_utc(),
        }),
    )
}
