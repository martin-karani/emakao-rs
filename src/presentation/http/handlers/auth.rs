use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use garde::Validate;

use crate::{
    application::{
        errors::AppError,
        use_cases::auth::{
            accept_invite::{AcceptInviteInput, AcceptInviteUseCase},
            change_password::{ChangePasswordInput, ChangePasswordUseCase},
            login::{PortalLoginInput, PortalLoginUseCase, StaffLoginInput, StaffLoginUseCase},
            refresh_token::RefreshInput,
        },
    },
    domain::{auth::AuthenticatedUser, enums::PortalType},
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        http::{
            dto::auth::{
                AcceptInviteDto, ChangePasswordDto, PortalLoginDto, RefreshDto, StaffLoginDto,
            },
            responses::auth::{LoginResponse, TokenResponse},
        },
    },
};

/// Staff login — POST app.emakao.co.ke/api/v1/auth/login
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = StaffLoginDto,
    responses(
        (status = 200, description = "Login successful",    body = LoginResponse),
        (status = 401, description = "Bad credentials",     body = ErrorResponse),
        (status = 422, description = "Validation error",    body = ErrorResponse),
    ),
    tag = "Auth"
)]
pub async fn staff_login(
    State(state): State<AppState>,
    Json(dto): Json<StaffLoginDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let out = StaffLoginUseCase::new(
        state.identity.auth_repo.clone(),
        state.identity.auth_port.clone(),
    )
    .execute(StaffLoginInput {
        agency_slug: dto.agency_slug,
        contact: dto.contact,
        password: dto.password,
        expiry_seconds: state.config.jwt_expiry_seconds,
    })
    .await?;

    Ok(Json(LoginResponse::from(out)))
}

/// Portal login (resident / owner / vendor).
/// Each portal's router captures `portal` as a constant in the closure.
pub async fn portal_login(
    portal: PortalType,
    state: AppState,
    dto: PortalLoginDto,
) -> Result<impl IntoResponse, AppError> {
    let out = PortalLoginUseCase::new(
        state.identity.auth_repo.clone(),
        state.identity.auth_port.clone(),
    )
    .execute(PortalLoginInput {
        portal,
        contact: dto.contact,
        password: dto.password,
        expiry_seconds: state.config.jwt_expiry_seconds,
    })
    .await?;

    Ok(Json(LoginResponse::from(out)))
}

/// Accept invite and set permanent password.
#[utoipa::path(
    post,
    path = "/api/v1/auth/accept-invite",
    request_body = AcceptInviteDto,
    responses(
        (status = 204, description = "Account activated"),
        (status = 404, description = "Token invalid or expired", body = ErrorResponse),
        (status = 422, description = "Validation error",         body = ErrorResponse),
    ),
    tag = "Auth"
)]
pub async fn accept_invite(
    State(state): State<AppState>,
    Json(dto): Json<AcceptInviteDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    AcceptInviteUseCase::new(
        state.identity.auth_repo.clone(),
        state.identity.auth_port.clone(),
    )
    .execute(AcceptInviteInput {
        token: dto.token,
        new_password: dto.new_password,
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Change password — requires a valid JWT (user must already be logged in).
#[utoipa::path(
    post,
    path = "/api/v1/auth/change-password",
    request_body = ChangePasswordDto,
    responses(
        (status = 204, description = "Password changed"),
        (status = 401, description = "Old password incorrect", body = ErrorResponse),
    ),
    tag = "Auth",
    security(("bearer_token" = []))
)]
pub async fn change_password(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(dto): Json<ChangePasswordDto>,
) -> Result<impl IntoResponse, AppError> {
    ChangePasswordUseCase::new(
        state.identity.auth_repo.clone(),
        state.identity.auth_port.clone(),
    )
    .execute(ChangePasswordInput {
        user_id: user.user_id,
        old_password: dto.old_password,
        new_password: dto.new_password,
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Refresh access token.
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    request_body = RefreshDto,
    responses(
        (status = 200, description = "Token refreshed", body = TokenResponse),
        (status = 401, description = "Invalid refresh token", body = ErrorResponse),
    ),
    tag = "Auth"
)]
pub async fn refresh(
    State(state): State<AppState>,
    Json(dto): Json<RefreshDto>,
) -> Result<impl IntoResponse, AppError> {
    let out = state
        .identity
        .auth_uc
        .refresh
        .execute(RefreshInput {
            refresh_token: dto.refresh_token,
            expiry_seconds: state.config.jwt_expiry_seconds,
        })
        .await?;

    Ok(Json(serde_json::json!({
        "access_token": out.access_token,
        "token_type":   out.token_type,
        "expires_in":   out.expires_in,
    })))
}
