use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;

use crate::presentation::app_state::AppState;

pub async fn require_admin(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    // Machine-to-machine callers (CI, Makefile, provisioning scripts).
    if let Some(provided_key) = req
        .headers()
        .get("x-admin-key")
        .and_then(|v| v.to_str().ok())
    {
        if let Some(ref key) = state.config.admin_api_key {
            if !key.is_empty() && provided_key == key {
                return next.run(req).await;
            }
        }
    }

    // Human operators with a `platform_admin` JWT.
    if let Some(token) = extract_bearer(&req) {
        if let Ok(claims) = state.identity.auth_port.verify_token(&token) {
            if claims.role == "platform_admin" {
                return next.run(req).await;
            }
        }
    }

    (
        StatusCode::FORBIDDEN,
        Json(json!({
            "error":   "FORBIDDEN",
            "message": "platform admin access required — \
                        provide X-Admin-Key header or a JWT with role=platform_admin"
        })),
    )
        .into_response()
}

fn extract_bearer(req: &Request<Body>) -> Option<String> {
    req.headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|t| t.trim().to_owned())
}
