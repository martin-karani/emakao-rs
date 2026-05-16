use axum::{routing::get, Router};

use crate::presentation::{app_state::AppState, http::handlers::analytics};

/// Analytics routes (staff-only, Growth+ plan).
/// Auth + subscription middleware applied by    the caller (build_staff_api).
///
/// Routes:
///   GET /api/v1/analytics/portfolio  – full portfolio analytics for a date range
///   GET /api/v1/analytics/revenue    – month-by-month revenue report
///   GET /api/v1/analytics/occupancy  – month-by-month occupancy trend
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/analytics/portfolio",
            get(analytics::portfolio_analytics),
        )
        .route("/api/v1/analytics/revenue", get(analytics::revenue_report))
        .route(
            "/api/v1/analytics/occupancy",
            get(analytics::occupancy_trends),
        )
}
