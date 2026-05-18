use axum::{
    routing::{get, patch, post},
    Router,
};

use crate::presentation::{
    app_state::AppState,
    http::handlers::disbursement::{
        create_disbursement, get_disbursement, get_my_disbursement, initiate_payout,
        list_disbursements, list_my_disbursements, update_disbursement_status,
    },
};

/// Staff routes — full CRUD + payout initiation.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/disbursements",
            get(list_disbursements).post(create_disbursement),
        )
        .route("/api/v1/disbursements/{id}", get(get_disbursement)) // :id → {id}
        .route(
            "/api/v1/disbursements/{id}/status", // :id → {id}
            patch(update_disbursement_status),
        )
        .route(
            "/api/v1/disbursements/{id}/initiate-payout", // :id → {id}
            post(initiate_payout),
        )
}

pub fn owner_portal_routes() -> Router<AppState> {
    Router::new()
        .route("/portal/disbursements", get(list_my_disbursements))
        .route("/portal/disbursements/{id}", get(get_my_disbursement))
}
