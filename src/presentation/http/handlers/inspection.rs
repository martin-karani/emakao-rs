// src/presentation/http/handlers/inspection.rs

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
        agency::ResolvedAgency,
        auth::AuthenticatedUser,
        enums::{InspectionStatus, InspectionType},
        inspection::{Inspection, InspectionItem},
    },
    infrastructure::db::inspection_repository_sqlx::PgInspectionRepo,
    presentation::{
        app_state::AppState, extractors::AgencyContext,
        http::responses::inspection::InspectionResponse,
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
    get,
    path = "/api/v1/inspections",
    params(ListInspectionsParams),
    responses(
        (status = 200, description = "Inspection list", body = Vec<InspectionResponse>),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Inspections",
    security(("bearer_token" = []))
)]
pub async fn list_inspections(
    ctx: AgencyContext,
    Query(params): Query<ListInspectionsParams>,
) -> Result<impl IntoResponse, AppError> {
    let repo = PgInspectionRepo::new(ctx.pool);
    let uc = ListInspectionsUseCase::new(std::sync::Arc::new(repo));

    let inspections: Vec<Inspection> = uc
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
    get,
    path = "/api/v1/inspections/{id}",
    params(("id" = Uuid, Path, description = "Inspection UUID")),
    responses(
        (status = 200, description = "Inspection found", body = InspectionResponse),
        (status = 404, description = "Not found"),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Inspections",
    security(("bearer_token" = []))
)]
pub async fn get_inspection(
    ctx: AgencyContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let repo = PgInspectionRepo::new(ctx.pool);
    let uc = GetInspectionUseCase::new(std::sync::Arc::new(repo));

    let inspection = uc.execute(id).await?;
    Ok(Json(InspectionResponse::from(inspection)))
}

/// POST /api/v1/inspections
#[utoipa::path(
    post,
    path = "/api/v1/inspections",
    request_body = CreateInspectionDto,
    responses(
        (status = 201, description = "Inspection scheduled", body = InspectionResponse),
        (status = 422, description = "Validation error"),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Inspections",
    security(("bearer_token" = []))
)]
pub async fn create_inspection(
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Json(dto): Json<CreateInspectionDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let repo = PgInspectionRepo::new(ctx.pool);
    let uc = CreateInspectionUseCase::new(std::sync::Arc::new(repo));

    let inspection = uc
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
    patch,
    path = "/api/v1/inspections/{id}",
    params(("id" = Uuid, Path, description = "Inspection UUID")),
    request_body = UpdateInspectionDto,
    responses(
        (status = 200, description = "Inspection updated", body = InspectionResponse),
        (status = 404, description = "Not found"),
        (status = 422, description = "Validation error"),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Inspections",
    security(("bearer_token" = []))
)]
pub async fn update_inspection(
    ctx: AgencyContext,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateInspectionDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let repo = PgInspectionRepo::new(ctx.pool);
    let uc = UpdateInspectionUseCase::new(std::sync::Arc::new(repo));

    let inspection = uc
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
    delete,
    path = "/api/v1/inspections/{id}",
    params(("id" = Uuid, Path, description = "Inspection UUID")),
    responses(
        (status = 204, description = "Inspection deleted"),
        (status = 404, description = "Not found"),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Inspections",
    security(("bearer_token" = []))
)]
pub async fn delete_inspection(
    ctx: AgencyContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let repo = PgInspectionRepo::new(ctx.pool);
    // Reuse find_by_id then delete — keeping logic flat rather than a dedicated UC
    repo.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
