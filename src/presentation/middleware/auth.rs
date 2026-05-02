use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;

use crate::{domain::auth::AuthenticatedUser, presentation::app_state::AppState};

pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let token = match extract_bearer(&req) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "MISSING_TOKEN",
                    "message": "Authorization header is missing or not a Bearer token"
                })),
            )
                .into_response();
        }
    };

    // Uses AppState::auth_port so JWT logic stays in the infrastructure layer
    let claims = match state.auth_port.verify_token(&token) {
        Ok(c) => c,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "INVALID_TOKEN",
                    "message": "token is invalid or has expired"
                })),
            )
                .into_response();
        }
    };

    req.extensions_mut().insert(AuthenticatedUser {
        user_id: claims.sub,
        agency_id: claims.agency_id,
        role: claims.role,
    });

    next.run(req).await
}

fn extract_bearer(req: &Request<Body>) -> Option<String> {
    req.headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|t| t.trim().to_owned())
}
