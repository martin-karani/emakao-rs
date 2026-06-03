// src/presentation/http/handlers/insights.rs
// REFACTORED: removed local `require_insights` helper and
// `Extension(sub): Extension<ResolvedSubscription>` from every handler.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Extension, Json,
};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::maintenance_repository::MaintenanceRepository,
        use_cases::insights::{
            maintenance_prediction::{
                ExpenseForecastUseCase, PredictiveMaintenanceUseCase, VendorAllocationUseCase,
            },
            rent_default_risk::RentDefaultRiskUseCase,
            tenant_churn::TenantChurnUseCase,
        },
    },
    domain::auth::AuthenticatedUser,
    infrastructure::db::{
        insight_repository_sqlx::PgInsightRepo, maintenance_repository_sqlx::PgMaintenanceRepo,
    },
    presentation::{
        app_state::AppState, error::ErrorResponse, extractors::AgencyContext,
        http::helpers::permission::check_permission, require_feature,
    },
};

// ── Query params ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, IntoParams)]
pub struct RiskScoreParams {
    pub min_score: Option<u8>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ChurnParams {
    /// Surface leases expiring within this many days. Default 90.
    pub horizon_days: Option<i64>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct MaintenanceAlertParams {
    /// Minimum historical work orders required to surface an alert. Default 2.
    pub min_historical_count: Option<i64>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ForecastParams {
    pub property_id: Option<Uuid>,
    /// Number of months to forecast (1–24). Default 6.
    pub horizon_months: Option<u8>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /api/v1/insights/rent-risk
#[utoipa::path(
    get,
    path = "/api/v1/insights/rent-risk",
    params(RiskScoreParams),
    responses(
        (status = 200, description = "Rent default risk scores"),
        (status = 402, description = "Plan upgrade required", body = ErrorResponse),
    ),
    tag = "Insights",
    security(("bearer_token" = []))
)]
pub async fn rent_default_risk(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<RiskScoreParams>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;


    let repo = Arc::new(PgInsightRepo::new(ctx.pool.clone()));
    let uc = RentDefaultRiskUseCase::new(repo);
    let scores = uc
        .execute(&ctx, params.limit.unwrap_or(50), params.offset.unwrap_or(0))
        .await?;
    Ok(Json(scores))
}

/// GET /api/v1/insights/churn
#[utoipa::path(
    get,
    path = "/api/v1/insights/churn",
    params(ChurnParams),
    responses(
        (status = 200, description = "Tenant churn predictions"),
        (status = 402, description = "Plan upgrade required", body = ErrorResponse),
    ),
    tag = "Insights",
    security(("bearer_token" = []))
)]
pub async fn tenant_churn(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<ChurnParams>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;


    let repo = Arc::new(PgInsightRepo::new(ctx.pool.clone()));
    let uc = TenantChurnUseCase::new(repo);
    let predictions = uc.execute(ctx.agency.id, params.horizon_days).await?;
    Ok(Json(predictions))
}

/// GET /api/v1/insights/maintenance-alerts
#[utoipa::path(
    get,
    path = "/api/v1/insights/maintenance-alerts",
    params(MaintenanceAlertParams),
    responses(
        (status = 200, description = "Predictive maintenance alerts"),
        (status = 402, description = "Plan upgrade required", body = ErrorResponse),
    ),
    tag = "Insights",
    security(("bearer_token" = []))
)]
pub async fn maintenance_alerts(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<MaintenanceAlertParams>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;


    let repo = Arc::new(PgInsightRepo::new(ctx.pool.clone()));
    let uc = PredictiveMaintenanceUseCase::new(repo);
    let alerts = uc
        .execute(ctx.agency.id, params.min_historical_count)
        .await?;
    Ok(Json(alerts))
}

/// GET /api/v1/insights/expense-forecast
#[utoipa::path(
    get,
    path = "/api/v1/insights/expense-forecast",
    params(ForecastParams),
    responses(
        (status = 200, description = "Expense forecast"),
        (status = 402, description = "Plan upgrade required", body = ErrorResponse),
    ),
    tag = "Insights",
    security(("bearer_token" = []))
)]
pub async fn expense_forecast(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<ForecastParams>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;


    let repo = Arc::new(PgInsightRepo::new(ctx.pool.clone()));
    let uc = ExpenseForecastUseCase::new(repo);
    let forecast = uc
        .execute(ctx.agency.id, params.property_id, params.horizon_months)
        .await?;
    Ok(Json(forecast))
}

/// GET /api/v1/insights/vendor-allocation/:work_order_id
#[utoipa::path(
    get,
    path = "/api/v1/insights/vendor-allocation/{work_order_id}",
    params(("work_order_id" = Uuid, Path, description = "Work order UUID")),
    responses(
        (status = 200, description = "Vendor recommendations"),
        (status = 402, description = "Plan upgrade required", body = ErrorResponse),
        (status = 404, description = "Work order not found",  body = ErrorResponse),
    ),
    tag = "Insights",
    security(("bearer_token" = []))
)]
pub async fn vendor_allocation(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Path(work_order_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;


    let maint_repo = PgMaintenanceRepo::new(ctx.pool.clone());
    let work_order = maint_repo
        .find_by_id(ctx.agency.id, work_order_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("work order {work_order_id}")))?;

    let repo = Arc::new(PgInsightRepo::new(ctx.pool.clone()));
    let uc = VendorAllocationUseCase::new(repo);
    let recommendations = uc
        .execute(ctx.agency.id, work_order_id, work_order.category.as_str())
        .await?;
    Ok(Json(recommendations))
}
