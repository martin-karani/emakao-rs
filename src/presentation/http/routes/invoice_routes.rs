use axum::{
    routing::{get, patch},
    Router,
};

use crate::presentation::{
    app_state::AppState,
    http::handlers::invoice::{create_invoice, get_invoice, list_invoices, update_invoice_status},
};

/// Staff routes — protected by the auth stack in router.rs
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/invoices", get(list_invoices).post(create_invoice))
        .route("/api/v1/invoices/{id}", get(get_invoice))
        .route("/api/v1/invoices/{id}/status", patch(update_invoice_status))
}
