use axum::{routing::get, Router};

use crate::presentation::{app_state::AppState, http::handlers::insights};

/// Insights routes (staff-only, Growth+ plan).
/// Auth + subscription middleware applied by the caller (build_staff_api).
///
/// Routes:
///   GET /api/v1/insights/rent-risk                   – rent default risk scoring
///   GET /api/v1/insights/churn                        – tenant churn prediction
///   GET /api/v1/insights/maintenance-alerts           – predictive maintenance alerts
///   GET /api/v1/insights/expense-forecast             – expense forecasting
///   GET /api/v1/insights/vendor-allocation/:id        – smart vendor recommendation
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/insights/rent-risk",
            get(insights::rent_default_risk),
        )
        .route("/api/v1/insights/churn", get(insights::tenant_churn))
        .route(
            "/api/v1/insights/maintenance-alerts",
            get(insights::maintenance_alerts),
        )
        .route(
            "/api/v1/insights/expense-forecast",
            get(insights::expense_forecast),
        )
        .route(
            "/api/v1/insights/vendor-allocation/:work_order_id",
            get(insights::vendor_allocation),
        )
}
