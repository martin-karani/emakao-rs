use axum::{
    body::Body,
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{domain::auth::AuthenticatedUser, presentation::app_state::AppState};

pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    // ── 1. Extract Bearer token ────────────────────────────────────────────────
    let token = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    let token = match token {
        Some(t) => t.to_owned(),
        None => return (StatusCode::UNAUTHORIZED, "missing authorization header").into_response(),
    };

    // ── 2. Decode & validate JWT ───────────────────────────────────────────────
    let claims = match state.jwt.verify_token(&token) {
        Ok(c) => c,
        Err(e) => {
            tracing::debug!(err = %e, "JWT verification failed");
            return (StatusCode::UNAUTHORIZED, "invalid or expired token").into_response();
        }
    };

    // ── 3. Check revocation blacklist (logout) ─────────────────────────────────
    match state.token_blacklist.is_revoked(&claims.jti).await {
        Ok(true) => {
            tracing::info!(jti = %claims.jti, "revoked token presented");
            return (StatusCode::UNAUTHORIZED, "token has been revoked").into_response();
        }
        Err(e) => {
            // If Redis is down, fail open with a warning rather than blocking
            // all users. Change to fail-closed if your threat model requires it.
            tracing::warn!(err = %e, "token blacklist unavailable — failing open");
        }
        Ok(false) => {}
    }

    // ── 3.5. Check iat against password change timestamp (Issue 18) ────────────
    match state.token_blacklist.is_revoked_by_iat(claims.sub, claims.iat).await {
        Ok(true) => {
            tracing::info!(user_id = %claims.sub, iat = %claims.iat, "token revoked by password change");
            return (StatusCode::UNAUTHORIZED, "token has been revoked due to password change").into_response();
        }
        Err(e) => {
            tracing::warn!(err = %e, "token blacklist iat check unavailable — failing open");
        }
        Ok(false) => {}
    }

    // ── 4. Build authenticated user and insert as extension ───────────────────
    let user = AuthenticatedUser {
        user_id: claims.sub,
        agency_id: claims.agency_id,
        role: claims.role,
        portal: claims.portal,
    };

    req.extensions_mut().insert(user);
    req.extensions_mut().insert(token);
    next.run(req).await
}
