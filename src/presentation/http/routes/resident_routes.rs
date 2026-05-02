use axum::{routing::get, Router};

use crate::presentation::{
    app_state::AppState,
    http::handlers::resident::{get_resident, invite_resident, list_residents},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/residents",
            get(list_residents).post(invite_resident),
        )
        .route("/api/v1/residents/:id", get(get_resident))
}
