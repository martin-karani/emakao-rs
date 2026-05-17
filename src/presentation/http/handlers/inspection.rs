use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use garde::Validate;
use serde::Deserialize;
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::inspection_repository::InspectionRepository,
        use_cases::inspection::{
            create_inspection::{CreateInspectionInput, CreateInspectionUseCase},
            get_inspection::GetInspectionUseCase,
            list_inspections::{ListInspectionsInput, ListInspectionsUseCase},
            update_inspection::{UpdateInspectionInput, UpdateInspectionUseCase},
        },
    },
    domain::{
        auth::AuthenticatedUser,
        enums::{InspectionStatus, InspectionType},
        inspection::{Inspection, InspectionItem},
    },
    infrastructure::db::inspection_repository_sqlx::PgInspectionRepo,
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        extractors::AgencyContext,
        http::{helpers::permission::check_permission, responses::inspection::InspectionResponse},
    },
};

// ── DTOs ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateInspectionDto {
    #[garde(skip)]
    pub property_id: Uuid,
    #[garde(skip)]
    pub unit_id: Uuid,
    #[garde(skip)]
    pub agreement_id: Option<Uuid>,
    #[garde(skip)]
    pub inspection_type: InspectionType,
    #[garde(skip)]
    pub scheduled_at: OffsetDateTime,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateInspectionDto {
    #[garde(skip)]
    pub status: Option<InspectionStatus>,
    #[garde(skip)]
    pub scheduled_at: Option<OffsetDateTime>,
    #[garde(skip)]
    pub completed_at: Option<OffsetDateTime>,
    #[garde(skip)]
    pub conducted_by: Option<Option<Uuid>>,
    #[garde(skip)]
    pub items: Option<Vec<InspectionItem>>,
    #[garde(length(max = 5000))]
    pub summary_notes: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListInspectionsParams {
    pub property_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub agreement_id: Option<Uuid>,
    pub status: Option<InspectionStatus>,
    pub inspection_type: Option<InspectionType>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /api/v1/inspections
#[utoipa::path(
    get, path = "/api/v1/inspections",
    params(ListInspectionsParams),
    responses(
        (status = 200, description = "Inspection list", body = Vec<InspectionResponse>),
        (status = 401, description = "Unauthorised",     body = ErrorResponse),
        (status = 403, description = "Forbidden",        body = ErrorResponse),
    ),
    tag = "Inspections", security(("bearer_token" = []))
)]
pub async fn list_inspections(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<ListInspectionsParams>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;

    let repo = Arc::new(PgInspectionRepo::new(ctx.pool));
    let inspections: Vec<Inspection> = ListInspectionsUseCase::new(repo)
        .execute(ListInspectionsInput {
            property_id: params.property_id,
            unit_id: params.unit_id,
            agreement_id: params.agreement_id,
            status: params.status,
            inspection_type: params.inspection_type,
            limit: params.limit,
            offset: params.offset,
        })
        .await?;

    Ok(Json(
        inspections
            .into_iter()
            .map(InspectionResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// GET /api/v1/inspections/:id
#[utoipa::path(
    get, path = "/api/v1/inspections/{id}",
    params(("id" = Uuid, Path, description = "Inspection UUID")),
    responses(
        (status = 200, description = "Inspection found", body = InspectionResponse),
        (status = 404, description = "Not found",         body = ErrorResponse),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Inspections", security(("bearer_token" = []))
)]
pub async fn get_inspection(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;

    let repo = Arc::new(PgInspectionRepo::new(ctx.pool));
    let inspection = GetInspectionUseCase::new(repo).execute(id).await?;
    Ok(Json(InspectionResponse::from(inspection)))
}

/// POST /api/v1/inspections
#[utoipa::path(
    post, path = "/api/v1/inspections",
    request_body = CreateInspectionDto,
    responses(
        (status = 201, description = "Inspection scheduled", body = InspectionResponse),
        (status = 422, description = "Validation error",      body = ErrorResponse),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Inspections", security(("bearer_token" = []))
)]
pub async fn create_inspection(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Json(dto): Json<CreateInspectionDto>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;
    dto.validate()?;

    let repo = Arc::new(PgInspectionRepo::new(ctx.pool));
    let inspection = CreateInspectionUseCase::new(repo)
        .execute(CreateInspectionInput {
            property_id: dto.property_id,
            unit_id: dto.unit_id,
            agreement_id: dto.agreement_id,
            inspection_type: dto.inspection_type,
            scheduled_at: dto.scheduled_at,
            created_by: user.user_id,
        })
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(InspectionResponse::from(inspection)),
    ))
}

/// PATCH /api/v1/inspections/:id
#[utoipa::path(
    patch, path = "/api/v1/inspections/{id}",
    params(("id" = Uuid, Path, description = "Inspection UUID")),
    request_body = UpdateInspectionDto,
    responses(
        (status = 200, description = "Inspection updated", body = InspectionResponse),
        (status = 404, description = "Not found",           body = ErrorResponse),
        (status = 422, description = "Validation error",    body = ErrorResponse),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Inspections", security(("bearer_token" = []))
)]
pub async fn update_inspection(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateInspectionDto>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;
    dto.validate()?;

    let repo = Arc::new(PgInspectionRepo::new(ctx.pool));
    let inspection = UpdateInspectionUseCase::new(repo)
        .execute(UpdateInspectionInput {
            id,
            status: dto.status,
            scheduled_at: dto.scheduled_at,
            completed_at: dto.completed_at,
            conducted_by: dto.conducted_by,
            items: dto.items,
            summary_notes: dto.summary_notes,
        })
        .await?;

    Ok(Json(InspectionResponse::from(inspection)))
}

/// DELETE /api/v1/inspections/:id
#[utoipa::path(
    delete, path = "/api/v1/inspections/{id}",
    params(("id" = Uuid, Path, description = "Inspection UUID")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Inspections", security(("bearer_token" = []))
)]
pub async fn delete_inspection(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;

    let repo = PgInspectionRepo::new(ctx.pool);
    repo.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
