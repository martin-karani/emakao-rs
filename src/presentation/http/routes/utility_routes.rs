use axum::{
    routing::{get, post},
    Router,
};

use crate::presentation::{
    app_state::AppState,
    http::handlers::utility::{create_meter, generate_bill, get_meter, record_reading},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/meters", post(create_meter))
        .route("/api/v1/meters/:id", get(get_meter))
        .route("/api/v1/meters/{id}/readings", post(record_reading))
        .route("/api/v1/meters/{id}/bills/generate", post(generate_bill))
}
