use axum::{
    routing::{get, post},
    Router,
};

use crate::presentation::{
    app_state::AppState,
    http::handlers::owner::{
        assign_owner_to_property, create_owner, get_my_profile, get_owner, invite_owner,
        list_my_disbursements, list_my_properties, list_owners, update_owner,
    },
};

pub fn staff_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/owners", get(list_owners).post(create_owner)) // POST uses directory-only version
        .route("/api/v1/owners/{id}", get(get_owner).put(update_owner))
        .route(
            "/api/v1/properties/{id}/owners",
            post(assign_owner_to_property),
        )
        .route("/api/v1/owners/invite", post(invite_owner))
}

pub fn owner_portal_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/owners/me", get(get_my_profile))
        .route("/api/v1/owners/me/properties", get(list_my_properties))
        .route(
            "/api/v1/owners/me/disbursements",
            get(list_my_disbursements),
        )
}
