use axum::{routing::get, Router};

use crate::presentation::{
    app_state::AppState,
    http::handlers::vendor::{create_vendor, get_vendor, list_vendors, update_vendor},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/vendors", get(list_vendors).post(create_vendor))
        .route("/api/v1/vendors/:id", get(get_vendor).put(update_vendor))
}
