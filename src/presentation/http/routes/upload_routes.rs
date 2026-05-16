// src/presentation/http/routes/upload_routes.rs

use axum::{routing::post, Router};

use crate::presentation::{app_state::AppState, http::handlers::upload::upload_file};

/// POST /api/v1/upload
/// Protected by the auth stack in router.rs.
pub fn routes() -> Router<AppState> {
    Router::new().route("/api/v1/upload", post(upload_file))
}
