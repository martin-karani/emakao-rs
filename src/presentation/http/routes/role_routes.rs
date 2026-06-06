use axum::{
    routing::{get, put},
    Router,
};

use crate::presentation::{app_state::AppState, http::handlers::role};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(role::list_roles).post(role::create_role))
        .route("/permissions", get(role::list_permissions))
        .route(
            "/{role_id}",
            put(role::update_role).delete(role::delete_role),
        )
}
