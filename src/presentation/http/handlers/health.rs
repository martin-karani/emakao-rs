use std::time::{Duration, Instant};

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use reqwest::Client;
use time::OffsetDateTime;
use tokio::time::timeout;

use crate::presentation::{
    app_state::AppState,
    http::responses::health::{
        ComponentStatus, ConfigurationStatus, DatabasePoolStatus, DependencyStatus, HealthResponse,
        ReadyResponse, RuntimeStatus, ServiceInfo,
    },
};

const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// Liveness probe — always 200 while the process is alive
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Process is alive", body = HealthResponse),
    ),
    tag = "Health"
)]
pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let report = build_report(&state).await;

    Json(HealthResponse {
        status: if dependencies_ready(&report.dependencies) {
            "ok"
        } else {
            "degraded"
        },
        service: service_info(&state),
        runtime: runtime_status(&state, report.timestamp),
        dependencies: report.dependencies,
        database_pool: database_pool_status(&state),
        configuration: configuration_status(&state),
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
    let report = build_report(&state).await;
    let ready = dependencies_ready(&report.dependencies);
    let status_code = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status_code,
        Json(ReadyResponse {
            status: if ready { "ready" } else { "not_ready" },
            ready,
            service: service_info(&state),
            runtime: runtime_status(&state, report.timestamp),
            dependencies: report.dependencies,
            database_pool: database_pool_status(&state),
            configuration: configuration_status(&state),
        }),
    )
}

// ── Internal helpers ──────────────────────────────────────────────────────────

struct HealthReport {
    timestamp: OffsetDateTime,
    dependencies: DependencyStatus,
}

async fn build_report(state: &AppState) -> HealthReport {
    let timestamp = OffsetDateTime::now_utc();
    let (database, redis, openfga) = tokio::join!(
        probe_database(state),
        probe_redis(state),
        probe_openfga(state),
    );

    HealthReport {
        timestamp,
        dependencies: DependencyStatus {
            database,
            redis,
            openfga,
        },
    }
}

async fn probe_database(state: &AppState) -> ComponentStatus {
    let started = Instant::now();
    let result = timeout(
        PROBE_TIMEOUT,
        sqlx::query("SELECT 1").execute(state.infra.tenant_pools.platform()),
    )
    .await;

    match result {
        Ok(Ok(_)) => ComponentStatus {
            ok: true,
            critical: true,
            latency_ms: Some(started.elapsed().as_millis()),
            detail: Some("platform postgres reachable".to_owned()),
        },
        Ok(Err(error)) => ComponentStatus {
            ok: false,
            critical: true,
            latency_ms: Some(started.elapsed().as_millis()),
            detail: Some(error.to_string()),
        },
        Err(_) => ComponentStatus {
            ok: false,
            critical: true,
            latency_ms: Some(started.elapsed().as_millis()),
            detail: Some("platform postgres probe timed out".to_owned()),
        },
    }
}

async fn probe_redis(state: &AppState) -> ComponentStatus {
    let started = Instant::now();
    let result = timeout(PROBE_TIMEOUT, state.infra.redis_cache.ping()).await;

    match result {
        Ok(Ok(response)) => ComponentStatus {
            ok: true,
            critical: true,
            latency_ms: Some(started.elapsed().as_millis()),
            detail: Some(format!("redis responded with {response}")),
        },
        Ok(Err(error)) => ComponentStatus {
            ok: false,
            critical: true,
            latency_ms: Some(started.elapsed().as_millis()),
            detail: Some(error.to_string()),
        },
        Err(_) => ComponentStatus {
            ok: false,
            critical: true,
            latency_ms: Some(started.elapsed().as_millis()),
            detail: Some("redis probe timed out".to_owned()),
        },
    }
}

async fn probe_openfga(state: &AppState) -> ComponentStatus {
    let started = Instant::now();
    let url = format!(
        "{}/stores?page_size=1",
        state.config.openfga_url.trim_end_matches('/')
    );
    let client = Client::new();
    let result = timeout(PROBE_TIMEOUT, client.get(&url).send()).await;

    match result {
        Ok(Ok(response)) if response.status().is_success() => ComponentStatus {
            ok: true,
            critical: true,
            latency_ms: Some(started.elapsed().as_millis()),
            detail: Some(format!("openfga reachable at {}", state.config.openfga_url)),
        },
        Ok(Ok(response)) => ComponentStatus {
            ok: false,
            critical: true,
            latency_ms: Some(started.elapsed().as_millis()),
            detail: Some(format!("openfga returned HTTP {}", response.status())),
        },
        Ok(Err(error)) => ComponentStatus {
            ok: false,
            critical: true,
            latency_ms: Some(started.elapsed().as_millis()),
            detail: Some(error.to_string()),
        },
        Err(_) => ComponentStatus {
            ok: false,
            critical: true,
            latency_ms: Some(started.elapsed().as_millis()),
            detail: Some("openfga probe timed out".to_owned()),
        },
    }
}

fn service_info(state: &AppState) -> ServiceInfo {
    ServiceInfo {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        port: state.config.port,
    }
}

fn runtime_status(state: &AppState, timestamp: OffsetDateTime) -> RuntimeStatus {
    RuntimeStatus {
        started_at: state.runtime.started_at,
        timestamp,
        uptime_seconds: state.runtime.started_instant.elapsed().as_secs() as i64,
        websocket_channels: state.runtime.websocket_channels.len(),
    }
}

fn database_pool_status(state: &AppState) -> DatabasePoolStatus {
    let pool = state.infra.tenant_pools.platform();

    DatabasePoolStatus {
        size: pool.size(),
        idle: pool.num_idle(),
        closed: pool.is_closed(),
        cached_tenant_pools: state.infra.tenant_pools.cached_agency_pools(),
    }
}

fn configuration_status(state: &AppState) -> ConfigurationStatus {
    ConfigurationStatus {
        admin_api_key_configured: has_value(state.config.admin_api_key.as_deref().unwrap_or_default()),
        smtp_configured: has_value(&state.config.smtp_host) && has_value(&state.config.smtp_from),
        mpesa_configured: has_value(&state.config.mpesa_consumer_key)
            && has_value(&state.config.mpesa_consumer_secret)
            && has_value(&state.config.mpesa_shortcode)
            && has_value(&state.config.mpesa_passkey)
            && has_value(&state.config.mpesa_callback_url),
        africa_talking_configured: has_value(&state.config.at_api_key)
            && has_value(&state.config.at_username),
        aws_s3_configured: has_value(&state.config.aws_access_key_id)
            && has_value(&state.config.aws_secret_access_key)
            && has_value(&state.config.aws_region)
            && has_value(&state.config.s3_bucket),
    }
}

fn dependencies_ready(dependencies: &DependencyStatus) -> bool {
    dependencies.database.ok && dependencies.redis.ok && dependencies.openfga.ok
}

fn has_value(value: &str) -> bool {
    !value.trim().is_empty()
}
