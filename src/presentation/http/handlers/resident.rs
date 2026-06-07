use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use garde::Validate;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::resident_repository::ResidentRepository,
        use_cases::resident::{
            get_resident::GetResidentUseCase,
            invite_resident::{InviteResidentInput, InviteResidentUseCase},
            list_residents::ListResidentsUseCase,
        },
    },
    domain::auth::AuthenticatedUser,
    infrastructure::db::resident_repository_sqlx::PgResidentRepo,
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        extractors::AgencyContext,
        http::{
            dto::{
                pagination::PaginationParams,
                resident::{InviteResidentDto, ListResidentsParams},
            },
            responses::{payment::PaymentClaimResponse, resident::ResidentResponse},
        },
    },
};

// ── Staff handlers ────────────────────────────────────────────────────────────

/// List residents (staff)
#[utoipa::path(
    get,
    path = "/api/v1/residents",
    params(ListResidentsParams),
    responses(
        (status = 200, description = "List of residents", body = [ResidentResponse]),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Residents",
    security(("bearer_token" = []))
)]
pub async fn list_residents(
    ctx: AgencyContext,
    Query(params): Query<ListResidentsParams>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgResidentRepo::from(ctx.pool));
    let usecase = ListResidentsUseCase::new(repo);
    let items = usecase
        .execute(
            ctx.agency.id,
            params.limit.unwrap_or(20).min(100),
            params.offset.unwrap_or(0),
        )
        .await?;
    Ok(Json(
        items
            .into_iter()
            .map(ResidentResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// Get resident by ID (staff)
#[utoipa::path(
    get,
    path = "/api/v1/residents/{id}",
    params(("id" = Uuid, Path, description = "Resident UUID")),
    responses(
        (status = 200, description = "Resident details", body = ResidentResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Resident not found", body = ErrorResponse),
    ),
    tag = "Residents",
    security(("bearer_token" = []))
)]
pub async fn get_resident(
    ctx: AgencyContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgResidentRepo::from(ctx.pool));
    let usecase = GetResidentUseCase::new(repo);
    let resident = usecase.execute(ctx.agency.id, id).await?;
    Ok(Json(ResidentResponse::from(resident)))
}

#[utoipa::path(
    post,
    path = "/api/v1/residents",
    request_body = InviteResidentDto,
    responses(
        (status = 201, description = "Resident invited", body = ResidentResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
    ),
    tag = "Residents",
    security(("bearer_token" = []))
)]
pub async fn invite_resident(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Json(dto): Json<InviteResidentDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let uc = InviteResidentUseCase {
        resident_repo: Arc::new(PgResidentRepo::from(ctx.pool)),
        auth_repo: state.identity.auth_repo.clone(),
        agency_repo: state.identity.agency_repo.clone(),
        auth_port: state.identity.auth_port.clone(),
        notifications: state.notifications.clone(),
    };

    let output = uc
        .execute(InviteResidentInput {
            agency_id: ctx.agency.id,
            agency_name: Some(ctx.agency.name),
            first_name: dto.first_name,
            last_name: dto.last_name,
            email: Some(dto.email),
            phone: dto.phone,
            national_id: dto.national_id,
            portal_base_url: "https://residents.emakao.co.ke".to_string(),
        })
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(ResidentResponse::from(output.resident)),
    ))
}

// ── Portal handlers ───────────────────────────────────────────────────────────

/// Get own profile (using JWT user_id)
#[utoipa::path(
    get,
    path = "/api/v1/residents/me",
    responses(
        (status = 200, description = "Current resident profile", body = ResidentResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Resident profile not found", body = ErrorResponse),
    ),
    tag = "Residents",
    security(("bearer_token" = []))
)]
pub async fn get_my_profile(
    Extension(user): Extension<AuthenticatedUser>,
    ctx: AgencyContext,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgResidentRepo::from(ctx.pool));
    let resident = ResidentRepository::find_by_user_id(repo.as_ref(), user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Resident profile not found".into()))?;
    Ok(Json(ResidentResponse::from(resident)))
}

#[utoipa::path(
    get,
    path = "/api/v1/residents/me/payments",
    params(PaginationParams),
    responses(
        (status = 200, description = "Current resident payment claims", body = [PaymentClaimResponse]),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Resident profile not found", body = ErrorResponse),
    ),
    tag = "Residents",
    security(("bearer_token" = []))
)]
pub async fn list_my_payments(
    Extension(user): Extension<AuthenticatedUser>,
    ctx: AgencyContext,
    Query(params): Query<PaginationParams>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgResidentRepo::from(ctx.pool));
    let resident = repo
        .find_by_user_id(user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Resident profile not found".into()))?;
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);
    let claims = repo
        .find_payment_claims_by_resident_id(resident.id, limit, offset)
        .await?;
    let responses: Vec<PaymentClaimResponse> = claims.into_iter().map(Into::into).collect();
    Ok(Json(responses))
}
