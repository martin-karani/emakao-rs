// src/presentation/http/handlers/ai_insights.rs

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
        use_cases::ai_insights::{
            maintenance_prediction::{
                ExpenseForecastUseCase, PredictiveMaintenanceUseCase, VendorAllocationUseCase,
            },
            rent_default_risk::RentDefaultRiskUseCase,
            tenant_churn::TenantChurnUseCase,
        },
    },
    domain::{auth::AuthenticatedUser, subscription::FeatureKey},
    presentation::{
        app_state::AppState, error::ErrorResponse, extractors::AgencyContext,
        http::helpers::permission::check_permission,
        middleware::subscription::ResolvedSubscription,
    },
};

// ── Feature-flag guard ────────────────────────────────────────────────────────

fn require_ai_insights(sub: &ResolvedSubscription) -> Result<(), AppError> {
    // "ai_insights" is a Growth+ feature; same gate as analytics.
    if !sub
        .entitlements
        .has_feature(&FeatureKey::Custom("analytics".to_string()))
    {
        return Err(AppError::PlanUpgradeRequired);
    }
    Ok(())
}

// ── Shared query params ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize, IntoParams)]
pub struct RiskScoreParams {
    /// Only return scores at or above this threshold (0–100). Default 0.
    pub min_score: Option<u8>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ChurnParams {
    /// Surface leases expiring within this many days. Default 90.
    pub horizon_days: Option<i64>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct MaintenanceAlertParams {
    /// Minimum number of historical work orders required to surface an alert. Default 2.
    pub min_historical_count: Option<i64>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ForecastParams {
    pub property_id: Option<Uuid>,
    /// Number of months to forecast (1–24). Default 6.
    pub horizon_months: Option<u8>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /api/v1/ai/rent-risk
///
/// Scores all active agreements for rent default risk.
/// Returns a ranked list (highest risk first). Requires Growth+ plan.
#[utoipa::path(
    get,
    path = "/api/v1/ai/rent-risk",
    params(RiskScoreParams),
    responses(
        (status = 200, description = "Rent default risk scores"),
        (status = 402, description = "Plan upgrade required", body = ErrorResponse),
    ),
    tag = "AI Insights",
    security(("bearer_token" = []))
)]
pub async fn rent_default_risk(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(sub): Extension<ResolvedSubscription>,
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
    require_ai_insights(&sub)?;

    let uc = RentDefaultRiskUseCase::new(Arc::new(ctx.pool));
    let scores = uc.execute(ctx.agency.id, params.min_score).await?;

    Ok(Json(scores))
}

/// GET /api/v1/ai/churn
///
/// Predicts tenant churn for leases approaching renewal. Requires Growth+ plan.
#[utoipa::path(
    get,
    path = "/api/v1/ai/churn",
    params(ChurnParams),
    responses(
        (status = 200, description = "Tenant churn predictions"),
        (status = 402, description = "Plan upgrade required", body = ErrorResponse),
    ),
    tag = "AI Insights",
    security(("bearer_token" = []))
)]
pub async fn tenant_churn(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(sub): Extension<ResolvedSubscription>,
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
    require_ai_insights(&sub)?;

    let uc = TenantChurnUseCase::new(Arc::new(ctx.pool));
    let predictions = uc.execute(ctx.agency.id, params.horizon_days).await?;

    Ok(Json(predictions))
}

/// GET /api/v1/ai/maintenance-alerts
///
/// Flags units likely to need maintenance soon based on historical work-order
/// frequency per category. Requires Growth+ plan.
#[utoipa::path(
    get,
    path = "/api/v1/ai/maintenance-alerts",
    params(MaintenanceAlertParams),
    responses(
        (status = 200, description = "Predictive maintenance alerts"),
        (status = 402, description = "Plan upgrade required", body = ErrorResponse),
    ),
    tag = "AI Insights",
    security(("bearer_token" = []))
)]
pub async fn maintenance_alerts(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(sub): Extension<ResolvedSubscription>,
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
    require_ai_insights(&sub)?;

    let uc = PredictiveMaintenanceUseCase::new(Arc::new(ctx.pool));
    let alerts = uc
        .execute(ctx.agency.id, params.min_historical_count)
        .await?;

    Ok(Json(alerts))
}

/// GET /api/v1/ai/expense-forecast
///
/// Forecasts property operating expenses for the next N months. Requires Growth+.
#[utoipa::path(
    get,
    path = "/api/v1/ai/expense-forecast",
    params(ForecastParams),
    responses(
        (status = 200, description = "Expense forecast"),
        (status = 402, description = "Plan upgrade required", body = ErrorResponse),
    ),
    tag = "AI Insights",
    security(("bearer_token" = []))
)]
pub async fn expense_forecast(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(sub): Extension<ResolvedSubscription>,
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
    require_ai_insights(&sub)?;

    let uc = ExpenseForecastUseCase::new(Arc::new(ctx.pool));
    let forecast = uc
        .execute(ctx.agency.id, params.property_id, params.horizon_months)
        .await?;

    Ok(Json(forecast))
}

/// GET /api/v1/ai/vendor-allocation/:work_order_id
///
/// Recommends the best vendor for an open work order based on category,
/// past performance, resolution speed, and cost. Requires Growth+.
#[utoipa::path(
    get,
    path = "/api/v1/ai/vendor-allocation/{work_order_id}",
    params(("work_order_id" = Uuid, Path, description = "Work order UUID")),
    responses(
        (status = 200, description = "Vendor recommendations"),
        (status = 402, description = "Plan upgrade required", body = ErrorResponse),
        (status = 404, description = "Work order not found",  body = ErrorResponse),
    ),
    tag = "AI Insights",
    security(("bearer_token" = []))
)]
pub async fn vendor_allocation(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(sub): Extension<ResolvedSubscription>,
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
    require_ai_insights(&sub)?;

    // Fetch the work order category.
    let category_row = sqlx::query!(
        "SELECT category::text AS category FROM work_orders WHERE id = $1",
        work_order_id
    )
    .fetch_optional(&ctx.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("work order {work_order_id}")))?;

    let category = category_row
        .category
        .unwrap_or_else(|| "general".to_string());

    let uc = VendorAllocationUseCase::new(Arc::new(ctx.pool));
    let rec = uc.execute(ctx.agency.id, work_order_id, &category).await?;

    Ok(Json(rec))
}
