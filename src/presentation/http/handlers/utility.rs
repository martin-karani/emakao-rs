// src/presentation/http/handlers/utility.rs
// REFACTORED: replaced `require_feature(&sub.entitlements, …)` with
// `require_feature!(state, ctx.agency.id, "utility_billing")`.
// Added `State(state): State<AppState>` to every handler that was missing it.
// Dropped `Extension(sub): Extension<ResolvedSubscription>` everywhere.

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
        use_cases::utility::{
            create_meter::{CreateMeterInput, CreateMeterUseCase},
            generate_bill::GenerateBillUseCase,
            get_meter::GetMeterUseCase,
            list_bills::ListBillsUseCase,
            list_meters::ListMetersUseCase,
            record_reading::{RecordReadingInput, RecordReadingUseCase},
        },
    },
    domain::auth::AuthenticatedUser,
    infrastructure::db::{
        billing_repository_sqlx::PgBillingRepo,
        utility_repository_sqlx::PgUtilityRepo,
    },
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        extractors::AgencyContext,
        http::{
            dto::utility::{CreateMeterDto, ListBillsParams, ListMetersParams, RecordReadingDto},
            responses::utility::{UtilityBillResponse, UtilityMeterResponse},
        },
        require_feature,
    },
};

// ── List meters ───────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/v1/meters",
    params(ListMetersParams),
    responses(
        (status = 200, description = "List of meters", body = Vec<UtilityMeterResponse>),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Utility",
    security(("bearer_token" = []))
)]
pub async fn list_meters(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Query(params): Query<ListMetersParams>,
) -> Result<impl IntoResponse, AppError> {
    require_feature!(state, ctx.agency.id, "utility_billing");

    let unit_id = params
        .unit_id
        .ok_or_else(|| AppError::Validation("unit_id query parameter is required".into()))?;

    let items = ListMetersUseCase::new(Arc::new(PgUtilityRepo::from(ctx.pool)))
        .execute(unit_id)
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(UtilityMeterResponse::from)
            .collect::<Vec<_>>(),
    ))
}

// ── Get meter ─────────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/v1/meters/{id}",
    params(("id" = Uuid, Path, description = "Meter UUID")),
    responses(
        (status = 200, description = "Meter details", body = UtilityMeterResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Meter not found",        body = ErrorResponse),
    ),
    tag = "Utility",
    security(("bearer_token" = []))
)]
pub async fn get_meter(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_feature!(state, ctx.agency.id, "utility_billing");

    let meter = GetMeterUseCase::new(Arc::new(PgUtilityRepo::from(ctx.pool)))
        .execute(id)
        .await?;

    Ok(Json(UtilityMeterResponse::from(meter)))
}

// ── Create meter ──────────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/v1/meters",
    request_body = CreateMeterDto,
    responses(
        (status = 201, description = "Meter created", body = UtilityMeterResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 422, description = "Validation error",       body = ErrorResponse),
    ),
    tag = "Utility",
    security(("bearer_token" = []))
)]
pub async fn create_meter(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Json(dto): Json<CreateMeterDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature!(state, ctx.agency.id, "utility_billing");

    let meter = CreateMeterUseCase::new(Arc::new(PgUtilityRepo::from(ctx.pool)))
        .execute(CreateMeterInput {
            unit_id: dto.unit_id,
            meter_type: dto.meter_type,
            billing_mode: dto.billing_mode,
            meter_number: dto.meter_number,
            rate_per_unit: dto.rate_per_unit,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(UtilityMeterResponse::from(meter))))
}

// ── Record reading ────────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/v1/meters/{id}/readings",
    params(("id" = Uuid, Path, description = "Meter UUID")),
    request_body = RecordReadingDto,
    responses(
        (status = 201, description = "Reading recorded"),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Meter not found",        body = ErrorResponse),
        (status = 422, description = "Validation error",       body = ErrorResponse),
    ),
    tag = "Utility",
    security(("bearer_token" = []))
)]
pub async fn record_reading(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<RecordReadingDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature!(state, ctx.agency.id, "utility_billing");

    RecordReadingUseCase::new(Arc::new(PgUtilityRepo::from(ctx.pool)))
        .execute(RecordReadingInput {
            meter_id: id,
            reading_value: dto.reading_value,
            recorded_by: user.user_id,
        })
        .await?;

    Ok(StatusCode::CREATED)
}

// ── List bills ────────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/v1/utility-bills",
    params(ListBillsParams),
    responses(
        (status = 200, description = "List of bills", body = Vec<UtilityBillResponse>),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
    ),
    tag = "Utility",
    security(("bearer_token" = []))
)]
pub async fn list_bills(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Query(params): Query<ListBillsParams>,
) -> Result<impl IntoResponse, AppError> {
    require_feature!(state, ctx.agency.id, "utility_billing");

    let unit_id = params
        .unit_id
        .ok_or_else(|| AppError::Validation("unit_id query parameter is required".into()))?;

    let items = ListBillsUseCase::new(Arc::new(PgUtilityRepo::from(ctx.pool)))
        .execute(
            unit_id,
            None,
            params.limit.unwrap_or(20).min(100),
            params.offset.unwrap_or(0),
        )
        .await?;

    Ok(Json(
        items
            .into_iter()
            .map(UtilityBillResponse::from)
            .collect::<Vec<_>>(),
    ))
}

// ── Generate bill ─────────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/v1/meters/{id}/bills/generate",
    params(("id" = Uuid, Path, description = "Meter UUID")),
    responses(
        (status = 200, description = "Bill generated",           body = UtilityBillResponse),
        (status = 401, description = "Missing or invalid JWT",   body = ErrorResponse),
        (status = 404, description = "Meter not found",          body = ErrorResponse),
        (status = 422, description = "Insufficient readings",    body = ErrorResponse),
    ),
    tag = "Utility",
    security(("bearer_token" = []))
)]
pub async fn generate_bill(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_feature!(state, ctx.agency.id, "utility_billing");

    let repo = Arc::new(PgUtilityRepo::from(ctx.pool.clone()));
    let billing_repo = Arc::new(PgBillingRepo::for_agency(ctx.pool, ctx.agency.id));

    let bill = GenerateBillUseCase::new(repo, billing_repo)
        .execute(id)
        .await?;

    Ok(Json(UtilityBillResponse::from(bill)))
}
