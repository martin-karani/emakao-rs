// src/presentation/http/handlers/dashboard.rs

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Extension, Json,
};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{
    application::{
        errors::AppError,
        use_cases::dashboard::get_dashboard::{GetDashboardInput, GetDashboardUseCase},
    },
    domain::auth::AuthenticatedUser,
    infrastructure::db::dashboard_repository_sqlx::PgDashboardRepo,
    presentation::{
        app_state::AppState, error::ErrorResponse, extractors::AgencyContext,
        http::helpers::permission::check_permission,
    },
};

#[derive(Debug, Deserialize, IntoParams)]
pub struct DashboardParams {
    /// How many days ahead to look for expiring leases (default 60, max 180).
    pub expiring_days: Option<i64>,
}

/// GET /api/v1/dashboard
///
/// Returns the multi-property dashboard summary: portfolio stats, per-property
/// occupancy, expiring leases, pending maintenance, and rent collection totals
/// for the current month.
#[utoipa::path(
    get,
    path = "/api/v1/dashboard",
    params(DashboardParams),
    responses(
        (status = 200, description = "Dashboard summary"),
        (status = 401, description = "Missing or invalid JWT",  body = ErrorResponse),
        (status = 402, description = "Subscription inactive",   body = ErrorResponse),
        (status = 403, description = "Insufficient permissions",body = ErrorResponse),
    ),
    tag = "Dashboard",
    security(("bearer_token" = []))
)]
pub async fn get_dashboard(
    State(state): State<AppState>,
    ctx: AgencyContext,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<DashboardParams>,
) -> Result<impl IntoResponse, AppError> {
    check_permission(
        &state,
        &ctx,
        &user,
        "staff",
        &format!("agency:{}", ctx.agency.id),
    )
    .await?;

    let repo = Arc::new(PgDashboardRepo::new(ctx.pool));
    let uc = GetDashboardUseCase::new(repo);

    let summary = uc
        .execute(GetDashboardInput {
            agency_id: ctx.agency.id,
            expiring_lease_days: params.expiring_days.map(|d| d.min(180)),
        })
        .await?;

    Ok(Json(summary))
}
