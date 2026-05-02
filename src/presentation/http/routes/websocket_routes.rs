use crate::presentation::{app_state::AppState, http::handlers::websocket::ws_handler};
use axum::{routing::get, Router};

pub fn routes() -> Router<AppState> {
    Router::new().route("/ws", get(ws_handler))
}
