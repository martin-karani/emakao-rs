use axum::{
    routing::{get, post},
    Router,
};

use crate::presentation::{
    app_state::AppState,
    http::handlers::payment::{list_claims, review_claim, submit_claim},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/payments", get(list_claims).post(submit_claim))
        .route("/api/v1/payments/{id}/review", post(review_claim))
}
