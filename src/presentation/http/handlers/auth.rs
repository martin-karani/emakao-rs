use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use garde::Validate;

use crate::{
    application::{
        errors::AppError,
        use_cases::auth::{
            login::LoginInput, refresh_token::RefreshInput, register::RegisterInput,
        },
    },
    domain::agency::ResolvedAgency,
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        http::{
            dto::auth::{LoginDto, RefreshDto, RegisterDto},
            responses::auth::TokenResponse,
        },
    },
};

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = LoginDto,
    responses(
        (status = 200, description = "Login successful",        body = TokenResponse),
        (status = 401, description = "Bad credentials",         body = ErrorResponse),
        (status = 422, description = "Validation error",        body = ErrorResponse),
    ),
    tag = "Auth"
)]
pub async fn login(
    State(state): State<AppState>,
    axum::Extension(agency): axum::Extension<ResolvedAgency>,
    Json(dto): Json<LoginDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let out = state
        .auth
        .login
        .execute(LoginInput {
            agency_id: agency.id,
            email: dto.email,
            password: dto.password,
            expiry_seconds: state.config.jwt_expiry_seconds,
        })
        .await?;

    Ok(Json(TokenResponse {
        access_token: out.access_token,
        token_type: out.token_type,
        expires_in: out.expires_in,
    }))
}

/// Register a new staff user under the current agency tenant
#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    request_body = RegisterDto,
    responses(
        (status = 201, description = "User registered"),
        (status = 409, description = "Email already exists",    body = ErrorResponse),
        (status = 422, description = "Validation error",        body = ErrorResponse),
    ),
    tag = "Auth"
)]
pub async fn register(
    State(state): State<AppState>,
    axum::Extension(agency): axum::Extension<ResolvedAgency>,
    Json(dto): Json<RegisterDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    state
        .auth
        .register
        .execute(RegisterInput {
            agency_id: agency.id,
            email: dto.email,
            password: dto.password,
            role: dto.role,
        })
        .await?;

    Ok(StatusCode::CREATED)
}

/// Exchange a refresh token for a new access token
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    request_body = RefreshDto,
    responses(
        (status = 200, description = "Token refreshed",         body = TokenResponse),
        (status = 401, description = "Invalid refresh token",   body = ErrorResponse),
    ),
    tag = "Auth"
)]
pub async fn refresh(
    State(state): State<AppState>,
    Json(dto): Json<RefreshDto>,
) -> Result<impl IntoResponse, AppError> {
    let out = state
        .auth
        .refresh
        .execute(RefreshInput {
            refresh_token: dto.refresh_token,
            expiry_seconds: state.config.jwt_expiry_seconds,
        })
        .await?;

    Ok(Json(TokenResponse {
        access_token: out.access_token,
        token_type: out.token_type,
        expires_in: out.expires_in,
    }))
}
