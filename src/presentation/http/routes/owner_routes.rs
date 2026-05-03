use axum::{
    routing::{get, post},
    Router,
};

use crate::presentation::{
    app_state::AppState,
    http::handlers::owner::{
        assign_owner_to_property, create_owner, get_owner, list_owners, update_owner,
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/owners", get(list_owners).post(create_owner))
        .route("/api/v1/owners/{id}", get(get_owner).put(update_owner))
        .route(
            "/api/v1/properties/{id}/owners",
            post(assign_owner_to_property),
        )
}
