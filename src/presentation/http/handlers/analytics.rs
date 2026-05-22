// src/presentation/http/handlers/analytics.rs
// REFACTORED: removed local `require_analytics` helper and
// `Extension(sub): Extension<ResolvedSubscription>` from every handler.
// Feature gating is now done via `require_feature!` against the
// EntitlementCache, which honours per-agency plan overrides.

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Extension, Json,
};
use serde::Deserialize;
use time::Date;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        use_cases::analytics::portfolio_analytics::{
            OccupancyTrendInput, OccupancyTrendUseCase, PortfolioAnalyticsInput,
            PortfolioAnalyticsUseCase, RevenueReportInput, RevenueReportUseCase,
        },
    },
    domain::auth::AuthenticatedUser,
    infrastructure::db::analytics_repository_sqlx::PgAnalyticsRepo,
    presentation::{
        app_state::AppState, error::ErrorResponse, extractors::AgencyContext,
        http::helpers::permission::check_permission, require_feature,
    },
};

// ── Query params ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, IntoParams)]
pub struct AnalyticsParams {
    /// Inclusive start date, e.g. `2025-01-01`.
    pub from: Date,
    /// Inclusive end date, e.g. `2025-12-31`.
    pub to: Date,
    /// Optionally filter to a single property.
    pub property_id: Option<Uuid>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /api/v1/analytics/portfolio
#[utoipa::path(
    get,
    path = "/api/v1/analytics/portfolio",
    params(AnalyticsParams),
    responses(
        (status = 200, description = "Portfolio analytics"),
        (status = 401, description = "Unauthorised",             body = ErrorResponse),
        (status = 402, description = "Upgrade required",         body = ErrorResponse),
        (status = 403, description = "Insufficient permissions", body = ErrorResponse),
    ),
    tag = "Analytics",
    security(("bearer_token" = []))
)]
pub async fn portfolio_analytics(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<AnalyticsParams>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;
    require_feature!(state, ctx.agency.id, "report_portfolio_summary");

    let repo = Arc::new(PgAnalyticsRepo::new(ctx.pool));
    let result = PortfolioAnalyticsUseCase { repo }
        .execute(PortfolioAnalyticsInput {
            agency_id: ctx.agency.id,
            period_start: params.from,
            period_end: params.to,
            property_id: params.property_id,
        })
        .await?;

    Ok(Json(result))
}

/// GET /api/v1/analytics/revenue
#[utoipa::path(
    get,
    path = "/api/v1/analytics/revenue",
    params(AnalyticsParams),
    responses(
        (status = 200, description = "Revenue report"),
        (status = 402, description = "Upgrade required", body = ErrorResponse),
    ),
    tag = "Analytics",
    security(("bearer_token" = []))
)]
pub async fn revenue_report(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<AnalyticsParams>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;
    require_feature!(state, ctx.agency.id, "report_portfolio_summary");

    let repo = Arc::new(PgAnalyticsRepo::new(ctx.pool));
    let result = RevenueReportUseCase { repo }
        .execute(RevenueReportInput {
            agency_id: ctx.agency.id,
            period_start: params.from,
            period_end: params.to,
            property_id: params.property_id,
        })
        .await?;

    Ok(Json(result))
}

/// GET /api/v1/analytics/occupancy
#[utoipa::path(
    get,
    path = "/api/v1/analytics/occupancy",
    params(AnalyticsParams),
    responses(
        (status = 200, description = "Occupancy trend"),
        (status = 402, description = "Upgrade required", body = ErrorResponse),
    ),
    tag = "Analytics",
    security(("bearer_token" = []))
)]
pub async fn occupancy_trends(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<AnalyticsParams>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;
    require_feature!(state, ctx.agency.id, "report_portfolio_summary");

    let repo = Arc::new(PgAnalyticsRepo::new(ctx.pool));
    let result = OccupancyTrendUseCase { repo }
        .execute(OccupancyTrendInput {
            agency_id: ctx.agency.id,
            period_start: params.from,
            period_end: params.to,
            property_id: params.property_id,
        })
        .await?;

    Ok(Json(result))
}
