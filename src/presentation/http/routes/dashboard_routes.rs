// src/presentation/http/routes/dashboard_routes.rs

use axum::{routing::get, Router};

use crate::presentation::http::handlers::dashboard;

/// Staff-only dashboard route.
/// Auth + subscription middleware are applied by the caller (build_staff_api).
pub fn routes() -> Router<crate::presentation::app_state::AppState> {
    Router::new().route("/api/v1/dashboard", get(dashboard::get_dashboard))
}
