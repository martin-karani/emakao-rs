use axum::{
    routing::{delete, get, patch, post},
    Router,
};

use crate::presentation::{
    app_state::AppState,
    http::handlers::inspection::{
        create_inspection, delete_inspection, get_inspection, list_inspections, update_inspection,
    },
};

/// Staff routes — all protected by the auth stack in router.rs
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/inspections",
            get(list_inspections).post(create_inspection),
        )
        .route(
            "/api/v1/inspections/:id",
            get(get_inspection)
                .patch(update_inspection)
                .delete(delete_inspection),
        )
}
