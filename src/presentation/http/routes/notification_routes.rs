use axum::{
    routing::{get, patch},
    Router,
};

use crate::presentation::{
    app_state::AppState,
    http::handlers::notification::{
        list_notifications, mark_all_notifications_read, mark_notification_read,
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/notifications", get(list_notifications))
        // NOTE: "read-all" must come before ":id/read" so Axum doesn't match
        // "read-all" as a UUID.
        .route(
            "/api/notifications/read-all",
            patch(mark_all_notifications_read),
        )
        .route(
            "/api/notifications/{id}/read",
            patch(mark_notification_read),
        )
}
