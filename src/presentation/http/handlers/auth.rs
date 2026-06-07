use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use garde::Validate;

use crate::{
    application::{
        errors::AppError,
        use_cases::auth::{
            accept_invite::{AcceptInviteInput, AcceptInviteUseCase},
            change_password::{ChangePasswordInput, ChangePasswordUseCase},
            forgot_password::{
                reset::{ResetPasswordInput, ResetPasswordUseCase},
                ForgotPasswordInput, ForgotPasswordUseCase,
            },
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
                AcceptInviteDto, ChangePasswordDto, ForgotPasswordDto, PortalLoginDto, RefreshDto,
                ResetPasswordDto, StaffLoginDto,
            },
            responses::auth::{LoginResponse, MeResponse, MessageResponse, TokenResponse},
        },
    },
};

/// Get current user info — GET /api/v1/auth/me
#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    responses(
        (status = 200, description = "Current user info", body = MeResponse),
        (status = 401, description = "Unauthorised",       body = ErrorResponse),
    ),
    tag = "Auth",
    security(("bearer_token" = []))
)]
pub async fn get_me(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<impl IntoResponse, AppError> {
    let stored_user = state
        .identity
        .auth_repo
        .find_user_by_id(user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    let agency = state
        .identity
        .agency_repo
        .find_agency_by_id(user.agency_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Agency not found".into()))?;

    Ok(Json(MeResponse {
        user_id: user.user_id,
        email: stored_user.email,
        phone: stored_user.phone,
        role: user.role,
        portal: user.portal,
        agency_id: agency.id,
        agency_name: agency.name,
        agency_slug: agency.slug,
    }))
}

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
        state.identity.agency_repo.clone(),
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
        state.identity.agency_repo.clone(),
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

/// POST /auth/logout
///
/// Revokes the calling user's JWT by adding its `jti` to the Redis blacklist.
/// The token remains technically valid until expiry but the middleware will
/// reject it on every subsequent request.
#[utoipa::path(
    post,
    path = "/auth/logout",
    responses(
        (status = 200, description = "Logged out successfully", body = MessageResponse),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Auth",
    security(("bearer_token" = []))
)]
pub async fn logout(
    State(state): State<AppState>,
    // `require_auth` middleware has already validated the token and injected
    // the raw token string as an extension (add it to the middleware if not done)
    Extension(raw_token): Extension<String>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<impl IntoResponse, AppError> {
    // Verify to get jti and expiry — we need the remaining TTL for Redis
    let claims = state.jwt.verify_token(&raw_token)?;

    let now_unix = time::OffsetDateTime::now_utc().unix_timestamp();
    let remaining = (claims.exp as i64).saturating_sub(now_unix);
    let ttl = remaining.max(0) as i64;

    // ISSUE 22 FIX: if the token is already past its expiry, ttl == 0.
    // SETEX with TTL=0 is a no-op (or immediately-expiring key) in Redis,
    // so we skip the write — the token is already invalid by expiry alone.
    if ttl > 0 {
        state
            .token_blacklist
            .revoke(&claims.jti, ttl)
            .await
            .map_err(|e| AppError::ExternalService(format!("blacklist: {e}")))?;
    }

    tracing::info!(user_id = %user.user_id, jti = %claims.jti, "user logged out");

    Ok(Json(MessageResponse {
        message: "Logged out successfully".into(),
    }))
}

/// POST /auth/forgot-password
///
/// Accepts an email address and, if it matches a known staff account, sends a
/// password-reset link. Always returns 200 to prevent user enumeration.
#[utoipa::path(
    post,
    path = "/auth/forgot-password",
    request_body = ForgotPasswordDto,
    responses(
        (status = 200, description = "Reset email sent if account exists", body = MessageResponse),
        (status = 422, description = "Validation error"),
    ),
    tag = "Auth"
)]
pub async fn forgot_password(
    State(state): State<AppState>,
    Json(dto): Json<ForgotPasswordDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let uc = ForgotPasswordUseCase::new(
        state.identity.auth_repo.clone(),
        state.notifications.clone(),
        state.config.app_base_url.clone(),
    );

    uc.execute(ForgotPasswordInput { email: dto.email }).await?;

    Ok(Json(MessageResponse {
        message: "If that email is registered, a reset link has been sent.".into(),
    }))
}

/// POST /auth/reset-password
///
/// Consumes the one-time reset token and sets the user's new password.
#[utoipa::path(
    post,
    path = "/auth/reset-password",
    request_body = ResetPasswordDto,
    responses(
        (status = 200, description = "Password updated successfully", body = MessageResponse),
        (status = 422, description = "Invalid or expired token / weak password"),
    ),
    tag = "Auth"
)]
pub async fn reset_password(
    State(state): State<AppState>,
    Json(dto): Json<ResetPasswordDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    ResetPasswordUseCase::new(
        state.identity.auth_repo.clone(),
        state.identity.auth_port.clone(),
    )
    .execute(ResetPasswordInput {
        token: dto.token,
        new_password: dto.new_password,
    })
    .await?;

    Ok(Json(MessageResponse {
        message: "Password updated successfully. Please log in with your new password.".into(),
    }))
}
