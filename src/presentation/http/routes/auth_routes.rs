use axum::{extract::State, middleware, routing::post, Json, Router};

use crate::{
    domain::enums::PortalType,
    presentation::{
        app_state::AppState,
        http::{
            dto::auth::PortalLoginDto,
            handlers::auth::{
                accept_invite, change_password, forgot_password, logout, portal_login, refresh,
                reset_password, staff_login,
            },
        },
        middleware::auth::require_auth,
    },
};

/// Staff login — POST /api/v1/auth/login  (requires agency_slug in body)
/// Also registers the shared refresh + accept-invite endpoints (registered once here).
pub fn staff_login_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/login", post(staff_login))
        .route("/api/v1/auth/refresh", post(refresh))
        .route("/api/v1/auth/accept-invite", post(accept_invite))
}

/// Resident portal login — POST /api/v1/auth/resident/login
pub fn resident_login_routes() -> Router<AppState> {
    Router::new().route(
        "/api/v1/auth/resident/login",
        post(
            |State(s): State<AppState>, Json(dto): Json<PortalLoginDto>| async move {
                portal_login(PortalType::Resident, s, dto).await
            },
        ),
    )
}

/// Owner portal login — POST /api/v1/auth/owner/login
pub fn owner_login_routes() -> Router<AppState> {
    Router::new().route(
        "/api/v1/auth/owner/login",
        post(
            |State(s): State<AppState>, Json(dto): Json<PortalLoginDto>| async move {
                portal_login(PortalType::Owner, s, dto).await
            },
        ),
    )
}

/// Vendor portal login — POST /api/v1/auth/vendor/login
pub fn vendor_login_routes() -> Router<AppState> {
    Router::new().route(
        "/api/v1/auth/vendor/login",
        post(
            |State(s): State<AppState>, Json(dto): Json<PortalLoginDto>| async move {
                portal_login(PortalType::Vendor, s, dto).await
            },
        ),
    )
}

/// Caretaker portal login — POST /api/v1/auth/caretaker/login
pub fn caretaker_login_routes() -> Router<AppState> {
    Router::new().route(
        "/api/v1/auth/caretaker/login",
        post(
            |State(s): State<AppState>, Json(dto): Json<PortalLoginDto>| async move {
                portal_login(PortalType::Caretaker, s, dto).await
            },
        ),
    )
}

pub fn change_password_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/change-password", post(change_password))
        .layer(middleware::from_fn_with_state(state, require_auth))
}

/// Public routes — no JWT required.
pub fn password_reset_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/forgot-password", post(forgot_password))
        .route("/auth/reset-password", post(reset_password))
}

/// Protected session route — JWT required (checked by `with_auth_stack`).
pub fn session_routes(_state: AppState) -> Router<AppState> {
    Router::new().route("/auth/logout", post(logout))
}
