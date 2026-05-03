use axum::{
    routing::{get, post},
    Router,
};

use crate::presentation::{
    app_state::AppState,
    http::handlers::agreement::{
        create_agreement, get_agreement, list_agreements, terminate_agreement,
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/agreements",
            get(list_agreements).post(create_agreement),
        )
        .route("/api/v1/agreements/{id}", get(get_agreement))
        .route(
            "/api/v1/agreements/{id}/terminate",
            post(terminate_agreement),
        )
}
