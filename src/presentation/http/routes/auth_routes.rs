use axum::{extract::State, middleware, routing::post, Json, Router};

use crate::{
    domain::enums::PortalType,
    presentation::{
        app_state::AppState,
        http::{
            dto::auth::PortalLoginDto,
            handlers::auth::{accept_invite, change_password, portal_login, refresh, staff_login},
        },
        middleware::auth::require_auth,
    },
};

pub fn staff_login_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/login", post(staff_login))
        .route("/api/v1/auth/refresh", post(refresh))
        .route("/api/v1/auth/accept-invite", post(accept_invite))
}

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

pub fn change_password_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/change-password", post(change_password))
        .layer(middleware::from_fn_with_state(state, require_auth))
}
