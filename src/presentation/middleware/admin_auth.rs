//! Platform-admin authentication middleware.
//!
//! Protects every `/api/v1/admin/*` route.  Two authentication paths are
//! supported so both automated tooling and human operators can call the API:
//!
//! ## Path 1 — static API key (machine-to-machine)
//! Set `ADMIN_API_KEY` in the environment and send it as:
//! ```
//! X-Admin-Key: <value>
//! ```
//!
//! ## Path 2 — JWT with `role = "platform_admin"`
//! Obtain a token whose `role` claim equals `"platform_admin"` and send it as:
//! ```
//! Authorization: Bearer <token>
//! ```
//!
//! Admin routes do **not** go through `tenant_resolver` — they are
//! platform-level operations that address agencies by UUID (path param), not
//! by slug.  Do not add `tenant_resolver` or `subscription_middleware` to any
//! router that uses `require_admin`.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;

use crate::presentation::app_state::AppState;

/// Axum middleware function.  Layer it on a `Router` with:
/// ```rust
/// .layer(middleware::from_fn_with_state(state.clone(), require_admin))
/// ```
pub async fn require_admin(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    // ── Path 1: static API key ────────────────────────────────────────────────
    // Allow machine-to-machine callers (CI, Makefile, provisioning scripts).
    if let Some(provided_key) = req
        .headers()
        .get("x-admin-key")
        .and_then(|v| v.to_str().ok())
    {
        let configured_key = &state.config.admin_api_key;
        // Guard against an accidentally empty key accepting any request.
        if !configured_key.is_empty() && provided_key == configured_key.as_str() {
            return next.run(req).await;
        }
    }

    // ── Path 2: JWT with platform_admin role ──────────────────────────────────
    // Allows human operators who hold a platform_admin JWT to call admin routes
    // without a second credential.
    if let Some(token) = extract_bearer(&req) {
        if let Ok(claims) = state.auth_port.verify_token(&token) {
            if claims.role == "platform_admin" {
                return next.run(req).await;
            }
        }
    }

    // ── Reject ────────────────────────────────────────────────────────────────
    (
        StatusCode::FORBIDDEN,
        Json(json!({
            "error": "FORBIDDEN",
            "message": "platform admin access required — \
                        provide X-Admin-Key header or a JWT with role=platform_admin"
        })),
    )
        .into_response()
}

/// Extract a Bearer token from the `Authorization` header, if present.
fn extract_bearer(req: &Request<Body>) -> Option<String> {
    req.headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|t| t.trim().to_owned())
}
