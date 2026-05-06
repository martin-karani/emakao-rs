use axum::{routing::get, Router};

use crate::presentation::{
    app_state::AppState,
    http::handlers::resident::{
        get_my_profile, get_resident, invite_resident, list_my_payments, list_residents,
    },
};

pub fn staff_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/residents",
            get(list_residents).post(invite_resident),
        )
        .route("/api/v1/residents/{id}", get(get_resident))
}

pub fn resident_portal_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/residents/me", get(get_my_profile))
        .route("/api/v1/residents/me/payments", get(list_my_payments))
}
