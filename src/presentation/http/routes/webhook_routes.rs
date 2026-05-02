use axum::{routing::post, Router};

use crate::presentation::{app_state::AppState, http::handlers::webhook::mpesa_callback};

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/v1/webhooks/mpesa", post(mpesa_callback))
}
