//! Health and readiness endpoints — no auth, no tenant resolution required.
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use time::OffsetDateTime;

use crate::presentation::app_state::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub timestamp: OffsetDateTime,
}

#[derive(Serialize)]
pub struct ReadyResponse {
    pub status: &'static str,
    pub database: ComponentStatus,
    pub timestamp: OffsetDateTime,
}

#[derive(Serialize)]
pub struct ComponentStatus {
    pub ok: bool,
    pub latency_ms: Option<u128>,
}

/// GET /health — liveness probe. Always returns 200 if the process is alive.
pub async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        timestamp: OffsetDateTime::now_utc(),
    })
}

/// GET /ready — readiness probe. Checks DB connectivity.
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