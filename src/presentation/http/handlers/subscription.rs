use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::{agency::ResolvedAgency, subscription::SubscriptionPlan},
    presentation::{app_state::AppState, middleware::subscription::ResolvedSubscription},
};

#[derive(Serialize)]
pub struct SubscriptionStatusResponse {
    pub is_active: bool,
    pub is_trial: bool,
    pub is_paid: bool,
    pub in_grace_period: bool,
    pub trial_days_remaining: i64,
    pub days_until_renewal: i64,
    pub plan_slug: String,
    pub plan_name: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct PlanResponse {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub price_kes: i32,
    pub yearly_price_kes: Option<i32>,
    pub trial_days: i32,
    pub sort_order: i32,
    pub metadata: Option<serde_json::Value>,
}

impl From<SubscriptionPlan> for PlanResponse {
    fn from(p: SubscriptionPlan) -> Self {
        Self {
            id: p.id,
            slug: p.slug,
            name: p.name,
            description: p.description,
            price_kes: p.price_kes,
            yearly_price_kes: p.yearly_price_kes,
            trial_days: p.trial_days,
            sort_order: p.sort_order,
            metadata: p.metadata,
        }
    }
}

#[derive(Deserialize)]
pub struct ChangePlanDto {
    pub plan_slug: String,
    pub mpesa_ref: Option<String>,
    pub custom_price: Option<i32>, // admin-only field
}

#[derive(Deserialize)]
pub struct CancelDto {
    pub reason: Option<String>,
}

#[derive(Deserialize)]
pub struct SetOverrideDto {
    pub feature_key: String,
    pub value: String, // "true" | "false" | custom string
    pub reason: Option<String>,
    pub expires_at: Option<OffsetDateTime>,
}

#[derive(Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

#[derive(serde::Deserialize, garde::Validate)]
pub struct InitiatePaymentDto {
    #[garde(length(min = 1))]
    pub plan_slug: String,
    /// Safaricom phone number, e.g. "254712345678".
    #[garde(length(min = 12, max = 13))]
    pub phone_number: String,
}

#[derive(serde::Serialize)]
pub struct InitiatePaymentResponse {
    pub checkout_request_id: String,
    pub message: String,
}

fn default_limit() -> i64 {
    20
}

// ═════════════════════════════════════════════════════════════════════════════
// Agency-facing handlers
// ═════════════════════════════════════════════════════════════════════════════

/// GET /api/v1/subscription/plans  — public, no auth required
pub async fn list_plans(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let plans = state.subscription.list_plans.execute().await?;
    Ok(Json(
        plans
            .into_iter()
            .map(PlanResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// GET /api/v1/subscription/plans/:slug  — public
pub async fn get_plan(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let plan = state.subscription.list_plans.get_by_slug(&slug).await?;
    Ok(Json(PlanResponse::from(plan)))
}

/// GET /api/v1/subscription  — requires active subscription
pub async fn get_status(Extension(sub): Extension<ResolvedSubscription>) -> impl IntoResponse {
    Json(SubscriptionStatusResponse {
        is_active: sub.state.is_active,
        is_trial: sub.state.is_trial,
        is_paid: sub.state.is_paid,
        in_grace_period: sub.state.in_grace_period,
        trial_days_remaining: sub.state.trial_days_remaining,
        days_until_renewal: sub.state.days_until_renewal,
        plan_slug: sub.state.plan_slug,
        plan_name: sub.state.plan_name,
        status: sub.state.status,
    })
}

/// GET /api/v1/subscription/entitlements
/// Returns the full feature + limits map so the frontend can drive UI gating
/// without per-feature round-trips (mirrors NestJS GET /subscription/entitlements).
pub async fn get_entitlements(
    Extension(sub): Extension<ResolvedSubscription>,
) -> impl IntoResponse {
    Json(sub.entitlements)
}

/// GET /api/v1/subscription/usage
/// Real counts vs. plan limits for every metered resource.
pub async fn get_usage(
    State(state): State<AppState>,
    Extension(sub): Extension<ResolvedSubscription>,
    Extension(agency): Extension<ResolvedAgency>,
) -> Result<impl IntoResponse, AppError> {
    let summary = state
        .subscription
        .get_usage
        .execute(agency.id, &sub.entitlements)
        .await?;
    Ok(Json(summary))
}

/// POST /api/v1/subscription/initiate-payment
pub async fn initiate_payment(
    axum::extract::State(state): axum::extract::State<crate::presentation::app_state::AppState>,
    axum::Extension(agency): axum::Extension<crate::domain::agency::ResolvedAgency>,
    axum::Json(dto): axum::Json<InitiatePaymentDto>,
) -> Result<impl axum::response::IntoResponse, crate::application::errors::AppError> {
    use garde::Validate;
    dto.validate()?;

    let output = state
        .subscription
        .initiate_payment
        .execute(
            crate::application::use_cases::subscription::initiate_subscription_payment::InitiatePaymentInput {
                agency_id: agency.id,
                plan_slug: dto.plan_slug,
                phone_number: dto.phone_number,
            },
        )
        .await?;

    Ok((
        axum::http::StatusCode::ACCEPTED,
        axum::Json(InitiatePaymentResponse {
            checkout_request_id: output.checkout_request_id,
            message: output.customer_message,
        }),
    ))
}

/// POST /api/v1/subscription/subscribe  — initial plan selection after signup
pub async fn subscribe(
    State(state): State<AppState>,
    Extension(agency): Extension<ResolvedAgency>,
    Json(dto): Json<ChangePlanDto>,
) -> Result<impl IntoResponse, AppError> {
    state
        .subscription
        .change_plan
        .execute(agency.id, &dto.plan_slug, dto.mpesa_ref.as_deref(), None)
        .await?;
    Ok(StatusCode::CREATED)
}

/// PATCH /api/v1/subscription/plan  — upgrade / downgrade
pub async fn change_plan(
    State(state): State<AppState>,
    Extension(agency): Extension<ResolvedAgency>,
    Json(dto): Json<ChangePlanDto>,
) -> Result<impl IntoResponse, AppError> {
    state
        .subscription
        .change_plan
        .execute(agency.id, &dto.plan_slug, dto.mpesa_ref.as_deref(), None)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/subscription  — cancel (stays active until period end)
pub async fn cancel_subscription(
    State(state): State<AppState>,
    Extension(agency): Extension<ResolvedAgency>,
    Json(dto): Json<CancelDto>,
) -> Result<impl IntoResponse, AppError> {
    state
        .subscription
        .cancel_subscription
        .execute(agency.id, dto.reason.as_deref())
        .await?;
    Ok(StatusCode::OK)
}

/// GET /api/v1/subscription/invoices
pub async fn list_invoices(
    State(state): State<AppState>,
    Extension(agency): Extension<ResolvedAgency>,
    Query(q): Query<PaginationQuery>,
) -> Result<impl IntoResponse, AppError> {
    let invoices = state
        .subscription
        .list_invoices
        .execute(agency.id, q.limit, q.offset)
        .await?;
    Ok(Json(invoices))
}

// ═════════════════════════════════════════════════════════════════════════════
// Admin handlers  (platform_admin / support roles only — protect at router level)
// ═════════════════════════════════════════════════════════════════════════════

/// GET /api/v1/admin/subscriptions/:agencyId
pub async fn admin_get_status(
    State(state): State<AppState>,
    Path(agency_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let state_val = state.subscription.get_state.execute(agency_id).await?;
    Ok(Json(state_val))
}

/// GET /api/v1/admin/subscriptions/:agencyId/entitlements
pub async fn admin_get_entitlements(
    State(state): State<AppState>,
    Path(agency_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let e = state
        .subscription
        .get_entitlements
        .execute(agency_id)
        .await?;
    Ok(Json(e))
}

/// PATCH /api/v1/admin/subscriptions/:agencyId/plan
pub async fn admin_change_plan(
    State(state): State<AppState>,
    Path(agency_id): Path<Uuid>,
    // Removed: Extension(actor): Extension<ResolvedAgency>
    // admin routes don't pass through tenant_resolver so ResolvedAgency is
    // never inserted — extracting it would panic at runtime.
    Json(dto): Json<ChangePlanDto>,
) -> Result<impl IntoResponse, AppError> {
    state
        .subscription
        .change_plan
        .execute(
            agency_id,
            &dto.plan_slug,
            dto.mpesa_ref.as_deref(),
            dto.custom_price,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/admin/subscriptions/:agencyId/overrides
pub async fn admin_set_override(
    State(state): State<AppState>,
    Path(agency_id): Path<Uuid>,
    // Removed: Extension(actor): Extension<ResolvedAgency>
    // Same reason as admin_change_plan above.
    Json(dto): Json<SetOverrideDto>,
) -> Result<impl IntoResponse, AppError> {
    state
        .subscription
        .set_override
        .execute(
            agency_id,
            &dto.feature_key,
            &dto.value,
            dto.reason.as_deref(),
            None, // overridden_by: platform admin has no agency UUID
            dto.expires_at,
        )
        .await?;
    Ok(StatusCode::CREATED)
}

/// DELETE /api/v1/admin/subscriptions/:agencyId/overrides/:featureKey
pub async fn admin_remove_override(
    State(state): State<AppState>,
    Path((agency_id, key)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse, AppError> {
    state
        .subscription
        .remove_override
        .execute(agency_id, &key)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/admin/subscriptions/:agencyId
pub async fn admin_cancel_subscription(
    State(state): State<AppState>,
    Path(agency_id): Path<Uuid>,
    Json(dto): Json<CancelDto>,
) -> Result<impl IntoResponse, AppError> {
    state
        .subscription
        .cancel_subscription
        .execute(agency_id, dto.reason.as_deref())
        .await?;
    Ok(StatusCode::OK)
}
