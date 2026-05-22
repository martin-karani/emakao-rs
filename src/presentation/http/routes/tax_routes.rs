// src/presentation/http/routes/tax_routes.rs

use axum::{
    routing::{get, post},
    Router,
};

use crate::presentation::app_state::AppState;
use crate::presentation::http::handlers::tax as handler;

pub fn routes() -> Router<AppState> {
    Router::new()
        // ── Compliance summary (dashboard widget) ─────────────────────────
        .route(
            "/tax/compliance-summary",
            get(handler::get_compliance_summary),
        )
        // ── Tax obligations ───────────────────────────────────────────────
        .route("/tax/obligations", get(handler::list_obligations))
        .route("/tax/obligations/:id/file", post(handler::file_obligation))
        .route("/tax/obligations/:id/pay", post(handler::mark_paid))
        // ── KRA PIN / TCC verification ────────────────────────────────────
        .route("/tax/kra/verify-pin", post(handler::verify_kra_pin))
}
