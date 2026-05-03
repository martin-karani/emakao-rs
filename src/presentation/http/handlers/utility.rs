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
    application::errors::AppError,
    application::use_cases::utility::{
        create_meter::{CreateMeterInput, CreateMeterUseCase},
        generate_bill::GenerateBillUseCase,
        get_meter::GetMeterUseCase,
        list_bills::ListBillsUseCase,
        list_meters::ListMetersUseCase,
        record_reading::{RecordReadingInput, RecordReadingUseCase},
    },
    domain::{auth::AuthenticatedUser, subscription::FeatureKey},
    infrastructure::db::utility_repository_sqlx::PgUtilityRepo,
    presentation::{
        app_state::AppState,
        error::ErrorResponse,
        extractors::TenantContext,
        http::{
            dto::utility::{CreateMeterDto, ListBillsParams, ListMetersParams, RecordReadingDto},
            responses::utility::{UtilityBillResponse, UtilityMeterResponse},
        },
        middleware::subscription::{require_feature, ResolvedSubscription},
    },
};

/// List utility meters
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
    State(_state): State<AppState>,
    ctx: TenantContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Query(params): Query<ListMetersParams>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::IntegUtilityBilling)?;

    let repo = Arc::new(PgUtilityRepo::from(ctx.pool));
    let usecase = ListMetersUseCase::new(repo);

    // ListMetersUseCase currently only takes unit_id. If unit_id is missing, it might need to list all for agency?
    // For now, if unit_id is provided, use it. Otherwise, this might need more repo support.
    let unit_id = params.unit_id.ok_or_else(|| {
        AppError::Validation("unit_id query parameter is required for now".into())
    })?;

    let items = usecase.execute(unit_id).await?;

    Ok(Json(
        items
            .into_iter()
            .map(UtilityMeterResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// Get meter details
#[utoipa::path(
    get,
    path = "/api/v1/meters/{id}",
    params(("id" = Uuid, Path, description = "Meter UUID")),
    responses(
        (status = 200, description = "Meter details", body = UtilityMeterResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Meter not found", body = ErrorResponse),
    ),
    tag = "Utility",
    security(("bearer_token" = []))
)]
pub async fn get_meter(
    State(_state): State<AppState>,
    ctx: TenantContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::IntegUtilityBilling)?;

    let repo = Arc::new(PgUtilityRepo::from(ctx.pool));
    let usecase = GetMeterUseCase::new(repo);

    let meter = usecase.execute(id).await?;
    Ok(Json(UtilityMeterResponse::from(meter)))
}

/// Create a new utility meter
#[utoipa::path(
    post,
    path = "/api/v1/meters",
    request_body = CreateMeterDto,
    responses(
        (status = 201, description = "Meter created", body = UtilityMeterResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
    ),
    tag = "Utility",
    security(("bearer_token" = []))
)]
pub async fn create_meter(
    State(_state): State<AppState>,
    ctx: TenantContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Json(dto): Json<CreateMeterDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature(&sub.entitlements, FeatureKey::IntegUtilityBilling)?;

    let repo = Arc::new(PgUtilityRepo::from(ctx.pool));
    let usecase = CreateMeterUseCase::new(repo);

    let meter = usecase
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

/// Record a new meter reading
#[utoipa::path(
    post,
    path = "/api/v1/meters/{id}/readings",
    params(("id" = Uuid, Path, description = "Meter UUID")),
    request_body = RecordReadingDto,
    responses(
        (status = 201, description = "Reading recorded"),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Meter not found", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
    ),
    tag = "Utility",
    security(("bearer_token" = []))
)]
pub async fn record_reading(
    State(_state): State<AppState>,
    ctx: TenantContext,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(id): Path<Uuid>,
    Json(dto): Json<RecordReadingDto>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()?;
    require_feature(&sub.entitlements, FeatureKey::IntegUtilityBilling)?;

    let repo = Arc::new(PgUtilityRepo::from(ctx.pool));
    let usecase = RecordReadingUseCase::new(repo);

    usecase
        .execute(RecordReadingInput {
            meter_id: id,
            reading_value: dto.reading_value,
            recorded_by: user.user_id,
        })
        .await?;

    Ok(StatusCode::CREATED)
}

/// List generated utility bills
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
    State(_state): State<AppState>,
    ctx: TenantContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Query(params): Query<ListBillsParams>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::IntegUtilityBilling)?;

    let repo = Arc::new(PgUtilityRepo::from(ctx.pool));
    let usecase = ListBillsUseCase::new(repo);

    // ListBillsUseCase currently only takes unit_id.
    let unit_id = params.unit_id.ok_or_else(|| {
        AppError::Validation("unit_id query parameter is required for now".into())
    })?;

    let items = usecase
        .execute(
            unit_id,
            None, // status filter not implemented in DTO yet
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

/// Manually trigger a bill generation for a meter
#[utoipa::path(
    post,
    path = "/api/v1/meters/{id}/bills/generate",
    params(("id" = Uuid, Path, description = "Meter UUID")),
    responses(
        (status = 200, description = "Bill generated", body = UtilityBillResponse),
        (status = 401, description = "Missing or invalid JWT", body = ErrorResponse),
        (status = 404, description = "Meter not found", body = ErrorResponse),
        (status = 422, description = "Insufficient readings", body = ErrorResponse),
    ),
    tag = "Utility",
    security(("bearer_token" = []))
)]
pub async fn generate_bill(
    State(_state): State<AppState>,
    ctx: TenantContext,
    Extension(sub): Extension<ResolvedSubscription>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_feature(&sub.entitlements, FeatureKey::IntegUtilityBilling)?;

    let repo = Arc::new(PgUtilityRepo::from(ctx.pool));
    let usecase = GenerateBillUseCase::new(repo);

    let bill = usecase.execute(id).await?;
    Ok(Json(UtilityBillResponse::from(bill)))
}
