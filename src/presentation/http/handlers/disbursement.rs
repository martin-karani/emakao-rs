// src/presentation/http/handlers/disbursement.rs

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use garde::Validate;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::sync::Arc;
use time::Date;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        use_cases::disbursement::{
            create_disbursement::{CreateDisbursementInput, CreateDisbursementUseCase},
            get_disbursement::GetDisbursementUseCase,
            initiate_payout::{InitiatePayoutInput, InitiatePayoutUseCase},
            list_disbursements::{ListDisbursementsInput, ListDisbursementsUseCase},
            update_disbursement_status::{
                UpdateDisbursementStatusInput, UpdateDisbursementStatusUseCase,
            },
        },
    },
    domain::{
        auth::AuthenticatedUser,
        enums::{DisbursementMethod, DisbursementStatus},
    },
    infrastructure::db::disbursement_repository_sqlx::PgDisbursementRepo,
    presentation::{
        extractors::AgencyContext, http::responses::disbursement::DisbursementResponse,
    },
};

// ── DTOs ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateDisbursementDto {
    #[garde(skip)]
    pub owner_id: Uuid,
    #[garde(skip)]
    pub property_id: Uuid,
    #[garde(range(min = 1.0))]
    pub amount_kes: Decimal,
    #[garde(skip)]
    pub method: DisbursementMethod,
    #[garde(skip)]
    pub period_start: Date,
    #[garde(skip)]
    pub period_end: Date,
    #[garde(length(max = 1000))]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateDisbursementStatusDto {
    #[garde(skip)]
    pub status: DisbursementStatus,
    #[garde(length(max = 200))]
    pub reference: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListDisbursementsParams {
    pub owner_id: Option<Uuid>,
    pub property_id: Option<Uuid>,
    pub status: Option<DisbursementStatus>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /api/v1/disbursements
#[utoipa::path(
    get, path = "/api/v1/disbursements",
    params(ListDisbursementsParams),
    responses(
        (status = 200, description = "Disbursement list", body = Vec<DisbursementResponse>),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Disbursements", security(("bearer_token" = []))
)]
pub async fn list_disbursements(
    ctx: AgencyContext,
    Query(params): Query<ListDisbursementsParams>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgDisbursementRepo::new(ctx.pool));
    let items = ListDisbursementsUseCase::new(repo)
        .execute(ListDisbursementsInput {
            agency_id: ctx.agency.id,
            owner_id: params.owner_id,
            property_id: params.property_id,
            status: params.status,
            limit: params.limit,
            offset: params.offset,
        })
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(DisbursementResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// GET /api/v1/disbursements/:id
#[utoipa::path(
    get, path = "/api/v1/disbursements/{id}",
    params(("id" = Uuid, Path, description = "Disbursement UUID")),
    responses(
        (status = 200, description = "Disbursement found", body = DisbursementResponse),
        (status = 404, description = "Not found"),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Disbursements", security(("bearer_token" = []))
)]
pub async fn get_disbursement(
    ctx: AgencyContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgDisbursementRepo::new(ctx.pool));
    let d = GetDisbursementUseCase::new(repo)
        .execute(ctx.agency.id, id)
        .await?;

    Ok(Json(DisbursementResponse::from(d)))
}

/// POST /api/v1/disbursements
#[utoipa::path(
    post, path = "/api/v1/disbursements",
    request_body = CreateDisbursementDto,
    responses(
        (status = 201, description = "Disbursement created", body = DisbursementResponse),
        (status = 422, description = "Validation error"),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Disbursements", security(("bearer_token" = []))
)]
pub async fn create_disbursement(
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Json(dto): Json<CreateDisbursementDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let repo = Arc::new(PgDisbursementRepo::new(ctx.pool));
    let d = CreateDisbursementUseCase::new(repo)
        .execute(CreateDisbursementInput {
            agency_id: ctx.agency.id,
            owner_id: dto.owner_id,
            property_id: dto.property_id,
            amount_kes: dto.amount_kes,
            method: dto.method,
            period_start: dto.period_start,
            period_end: dto.period_end,
            notes: dto.notes,
            created_by: user.user_id,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(DisbursementResponse::from(d))))
}

/// PATCH /api/v1/disbursements/:id/status
#[utoipa::path(
    patch, path = "/api/v1/disbursements/{id}/status",
    params(("id" = Uuid, Path, description = "Disbursement UUID")),
    request_body = UpdateDisbursementStatusDto,
    responses(
        (status = 200, description = "Status updated", body = DisbursementResponse),
        (status = 404, description = "Not found"),
        (status = 422, description = "Invalid status transition"),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Disbursements", security(("bearer_token" = []))
)]
pub async fn update_disbursement_status(
    ctx: AgencyContext,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateDisbursementStatusDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;

    let repo = Arc::new(PgDisbursementRepo::new(ctx.pool));
    let d = UpdateDisbursementStatusUseCase::new(repo)
        .execute(UpdateDisbursementStatusInput {
            agency_id: ctx.agency.id,
            id,
            status: dto.status,
            reference: dto.reference,
        })
        .await?;

    Ok(Json(DisbursementResponse::from(d)))
}

/// POST /api/v1/disbursements/:id/initiate-payout
#[utoipa::path(
    post, path = "/api/v1/disbursements/{id}/initiate-payout",
    params(("id" = Uuid, Path, description = "Disbursement UUID")),
    responses(
        (status = 200, description = "Payout initiated — status set to Processing", body = DisbursementResponse),
        (status = 404, description = "Not found"),
        (status = 422, description = "Disbursement not in Pending status"),
        (status = 401, description = "Unauthorised"),
    ),
    tag = "Disbursements", security(("bearer_token" = []))
)]
pub async fn initiate_payout(
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let repo = Arc::new(PgDisbursementRepo::new(ctx.pool));
    let d = InitiatePayoutUseCase::new(repo)
        .execute(InitiatePayoutInput {
            agency_id: ctx.agency.id,
            disbursement_id: id,
            initiated_by: user.user_id,
        })
        .await?;

    Ok(Json(DisbursementResponse::from(d)))
}
