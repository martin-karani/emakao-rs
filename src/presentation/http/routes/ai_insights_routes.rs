// src/presentation/http/routes/ai_insights_routes.rs

use axum::{routing::get, Router};

use crate::presentation::{app_state::AppState, http::handlers::ai_insights};

/// AI Insights routes (staff-only, Growth+ plan).
/// Auth + subscription middleware applied by the caller (build_staff_api).
///
/// Routes:
///   GET /api/v1/ai/rent-risk                   – rent default risk scoring
///   GET /api/v1/ai/churn                        – tenant churn prediction
///   GET /api/v1/ai/maintenance-alerts           – predictive maintenance alerts
///   GET /api/v1/ai/expense-forecast             – expense forecasting
///   GET /api/v1/ai/vendor-allocation/:id        – smart vendor recommendation
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/ai/rent-risk", get(ai_insights::rent_default_risk))
        .route("/api/v1/ai/churn", get(ai_insights::tenant_churn))
        .route(
            "/api/v1/ai/maintenance-alerts",
            get(ai_insights::maintenance_alerts),
        )
        .route(
            "/api/v1/ai/expense-forecast",
            get(ai_insights::expense_forecast),
        )
        .route(
            "/api/v1/ai/vendor-allocation/:work_order_id",
            get(ai_insights::vendor_allocation),
        )
}
