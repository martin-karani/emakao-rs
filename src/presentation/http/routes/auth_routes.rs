use axum::{routing::post, Router};

use crate::presentation::{
    app_state::AppState,
    http::handlers::auth::{login, refresh, register},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/refresh", post(refresh))
}
