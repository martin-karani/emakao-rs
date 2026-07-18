use axum::{
    routing::{get, post, put},
    Router,
};

use crate::presentation::app_state::AppState;
use crate::presentation::http::handlers::feedback::{
    create_feedback, create_feedback_reply, get_feedback, list_feedback, list_feedback_replies,
    update_feedback_status,
};

pub fn feedback_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/feedback", post(create_feedback))
        .route("/api/v1/feedback", get(list_feedback))
        .route("/api/v1/feedback/{feedback_id}", get(get_feedback))
        .route("/api/v1/feedback/{feedback_id}/status", put(update_feedback_status))
        .route("/api/v1/feedback/{feedback_id}/replies", post(create_feedback_reply))
        .route("/api/v1/feedback/{feedback_id}/replies", get(list_feedback_replies))
}
