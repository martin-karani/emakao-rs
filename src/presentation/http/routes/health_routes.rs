use axum::{routing::get, Router};

use crate::presentation::{
    app_state::AppState,
    http::handlers::health::{health, ready},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
}
