use axum::{routing::get, Router};

use crate::presentation::{app_state::AppState, http::handlers::document};

/// Document CRUD routes (staff-only).
/// Auth + subscription middleware applied by the caller (build_staff_api).
///
/// Routes:
///   GET    /api/v1/documents          – list (with optional filters)
///   POST   /api/v1/documents          – multipart upload
///   GET    /api/v1/documents/:id      – fetch one document
///   DELETE /api/v1/documents/:id      – soft-delete + S3 removal
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/documents",
            get(document::list_documents).post(document::upload_document),
        )
        .route(
            "/api/v1/documents/:id",
            get(document::get_document).delete(document::delete_document),
        )
}
