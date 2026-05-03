use axum::{extract::State, middleware, routing::post, Json, Router};

use crate::{
    domain::auth::PortalType,
    presentation::{
        app_state::AppState,
        http::{
            dto::auth::PortalLoginDto,
            handlers::auth::{accept_invite, change_password, portal_login, refresh, staff_login},
        },
        middleware::auth::require_auth,
    },
};

/// Staff login (app.emakao.co.ke)
pub fn staff_login_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/login", post(staff_login))
        .route("/api/v1/auth/refresh", post(refresh))
        .route("/api/v1/auth/accept-invite", post(accept_invite))
}

/// Resident portal login (residents.emakao.co.ke)
pub fn resident_login_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/auth/login",
            post(
                |State(s): State<AppState>, Json(dto): Json<PortalLoginDto>| async move {
                    portal_login(PortalType::Resident, s, dto).await
                },
            ),
        )
        .route("/api/v1/auth/refresh", post(refresh))
        .route("/api/v1/auth/accept-invite", post(accept_invite))
}

/// Owner portal login (owners.emakao.co.ke)
pub fn owner_login_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/auth/login",
            post(
                |State(s): State<AppState>, Json(dto): Json<PortalLoginDto>| async move {
                    portal_login(PortalType::Owner, s, dto).await
                },
            ),
        )
        .route("/api/v1/auth/refresh", post(refresh))
        .route("/api/v1/auth/accept-invite", post(accept_invite))
}

/// Vendor portal login (vendors.emakao.co.ke)
pub fn vendor_login_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/auth/login",
            post(
                |State(s): State<AppState>, Json(dto): Json<PortalLoginDto>| async move {
                    portal_login(PortalType::Vendor, s, dto).await
                },
            ),
        )
        .route("/api/v1/auth/refresh", post(refresh))
        .route("/api/v1/auth/accept-invite", post(accept_invite))
}

/// Change-password route is shared across all portals.
/// Requires auth — user must already be logged in with a valid (possibly
/// must_change_password=true) JWT.
pub fn change_password_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/change-password", post(change_password))
        .layer(middleware::from_fn_with_state(state, require_auth))
}
