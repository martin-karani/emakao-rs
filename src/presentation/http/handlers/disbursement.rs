use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use garde::Validate;
use rust_decimal::Decimal;
use std::sync::Arc;
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
    domain::auth::AuthenticatedUser,
    infrastructure::db::disbursement_repository_sqlx::PgDisbursementRepo,
    presentation::{
        extractors::AgencyContext,
        http::{
            dto::disbursement::{
                CreateDisbursementDto, ListDisbursementsParams, UpdateDisbursementStatusDto,
            },
            responses::disbursement::DisbursementResponse,
        },
    },
};

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

    // Amount guard — done here because garde cannot validate Decimal ranges.
    if dto.amount_kes <= Decimal::ZERO {
        return Err(AppError::Validation(
            "amount_kes must be greater than zero".into(),
        ));
    }

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
        (status = 200, description = "Payout initiated", body = DisbursementResponse),
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

/// GET /portal/disbursements
/// Lists disbursements that belong to the calling owner.
pub async fn list_my_disbursements(
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Query(p): Query<ListDisbursementsParams>,
) -> Result<impl IntoResponse, AppError> {
    let repo = std::sync::Arc::new(PgDisbursementRepo::new(ctx.pool));
    let disbursements = ListDisbursementsUseCase::new(repo)
        .execute(ListDisbursementsInput {
            agency_id: ctx.agency.id,
            // Force filter to the JWT owner — ignores any owner_id in query params
            owner_id: Some(user.user_id),
            property_id: p.property_id,
            status: p.status,
            limit: p.limit,
            offset: p.offset,
        })
        .await?;

    Ok(Json(
        disbursements
            .into_iter()
            .map(DisbursementResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// GET /portal/disbursements/:id
pub async fn get_my_disbursement(
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let repo = std::sync::Arc::new(PgDisbursementRepo::new(ctx.pool));
    let d = GetDisbursementUseCase::new(repo)
        .execute(ctx.agency.id, id)
        .await?;

    // Ownership check — owners can only see their own disbursements
    if d.owner_id != user.user_id {
        return Err(AppError::NotFound(format!("disbursement {id}")));
    }

    Ok(Json(DisbursementResponse::from(d)))
}
