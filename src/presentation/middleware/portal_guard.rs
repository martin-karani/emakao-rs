// Rejects any request whose JWT portal does not match the route group it is
// hitting. Applied per route group at router build time, not globally.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;

use crate::domain::{auth::AuthenticatedUser, enums::PortalType};

pub async fn portal_guard(required: PortalType, req: Request<Body>, next: Next) -> Response {
    let actual = req
        .extensions()
        .get::<AuthenticatedUser>()
        .map(|u| u.portal);

    match actual {
        Some(p) if p == required => next.run(req).await,
        Some(actual_portal) => (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "WRONG_PORTAL",
                "message": format!(
                    "This token is for the '{}' portal; you are accessing the '{}' portal.",
                    actual_portal, required
                )
            })),
        )
            .into_response(),
        None => {
            // Should never happen — require_auth runs first.
            tracing::error!("portal_guard: AuthenticatedUser missing");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
