use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::{agency::ResolvedAgency, subscription::SubscriptionPlan},
    presentation::{
        app_state::AppState, error::ErrorResponse, middleware::subscription::ResolvedSubscription,
    },
};

#[derive(Serialize, ToSchema)]
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

#[derive(Serialize, ToSchema)]
pub struct PlanResponse {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub price_kes: i32,
    pub yearly_price_kes: Option<i32>,
    pub trial_days: i32,
    pub sort_order: i32,
    /// Arbitrary JSON metadata blob
    #[schema(value_type = Object, nullable = true)]
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

#[derive(Deserialize, ToSchema)]
pub struct ChangePlanDto {
    pub plan_slug: String,
    pub mpesa_ref: Option<String>,
    /// Admin-only: override the plan price
    pub custom_price: Option<i32>,
}

#[derive(Deserialize, ToSchema)]
pub struct CancelDto {
    pub reason: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct SetOverrideDto {
    pub feature_key: String,
    /// `"true"`, `"false"`, or a custom string value
    pub value: String,
    pub reason: Option<String>,
    pub expires_at: Option<OffsetDateTime>,
}

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct PaginationQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

#[derive(Deserialize, garde::Validate, ToSchema)]
pub struct InitiatePaymentDto {
    #[garde(length(min = 1))]
    pub plan_slug: String,
    /// Safaricom phone number, e.g. `"254712345678"`
    #[garde(length(min = 12, max = 13))]
    pub phone_number: String,
}

#[derive(Serialize, ToSchema)]
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

/// List all available subscription plans (public, no auth required)
#[utoipa::path(
    get,
    path = "/api/v1/subscription/plans",
    responses(
        (status = 200, description = "List of plans",               body = Vec<PlanResponse>),
    ),
    tag = "Subscription"
)]
pub async fn list_plans(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let plans = state.subscription.list_plans.execute().await?;
    Ok(Json(
        plans
            .into_iter()
            .map(PlanResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// Get a single subscription plan by slug
#[utoipa::path(
    get,
    path = "/api/v1/subscription/plans/{slug}",
    params(("slug" = String, Path, description = "Plan slug, e.g. `professional`")),
    responses(
        (status = 200, description = "Plan found",                  body = PlanResponse),
        (status = 404, description = "Plan not found",              body = ErrorResponse),
    ),
    tag = "Subscription"
)]
pub async fn get_plan(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let plan = state.subscription.list_plans.get_by_slug(&slug).await?;
    Ok(Json(PlanResponse::from(plan)))
}

/// Get the current agency's subscription status
#[utoipa::path(
    get,
    path = "/api/v1/subscription",
    responses(
        (status = 200, description = "Subscription status",         body = SubscriptionStatusResponse),
        (status = 401, description = "Missing or invalid JWT",      body = ErrorResponse),
    ),
    tag = "Subscription",
    security(("bearer_token" = []))
)]
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

/// Get full feature + limits entitlements map
#[utoipa::path(
    get,
    path = "/api/v1/subscription/entitlements",
    responses(
        (status = 200, description = "Entitlements map (opaque JSON object)"),
        (status = 401, description = "Missing or invalid JWT",      body = ErrorResponse),
    ),
    tag = "Subscription",
    security(("bearer_token" = []))
)]
pub async fn get_entitlements(
    Extension(sub): Extension<ResolvedSubscription>,
) -> impl IntoResponse {
    Json(sub.entitlements)
}

/// Get real resource usage vs plan limits
#[utoipa::path(
    get,
    path = "/api/v1/subscription/usage",
    responses(
        (status = 200, description = "Usage summary (opaque JSON object)"),
        (status = 401, description = "Missing or invalid JWT",      body = ErrorResponse),
    ),
    tag = "Subscription",
    security(("bearer_token" = []))
)]
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

/// Initiate an M-Pesa STK push to pay for a subscription plan
#[utoipa::path(
    post,
    path = "/api/v1/subscription/initiate-payment",
    request_body = InitiatePaymentDto,
    responses(
        (status = 202, description = "STK push initiated",          body = InitiatePaymentResponse),
        (status = 401, description = "Missing or invalid JWT",      body = ErrorResponse),
        (status = 422, description = "Validation error",            body = ErrorResponse),
    ),
    tag = "Subscription",
    security(("bearer_token" = []))
)]
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

/// Select an initial subscription plan after signup
#[utoipa::path(
    post,
    path = "/api/v1/subscription/subscribe",
    request_body = ChangePlanDto,
    responses(
        (status = 201, description = "Plan selected"),
        (status = 401, description = "Missing or invalid JWT",      body = ErrorResponse),
    ),
    tag = "Subscription",
    security(("bearer_token" = []))
)]
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

/// Upgrade or downgrade the current subscription plan
#[utoipa::path(
    patch,
    path = "/api/v1/subscription/plan",
    request_body = ChangePlanDto,
    responses(
        (status = 204, description = "Plan changed"),
        (status = 401, description = "Missing or invalid JWT",      body = ErrorResponse),
    ),
    tag = "Subscription",
    security(("bearer_token" = []))
)]
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

/// Cancel the current subscription (stays active until period end)
#[utoipa::path(
    delete,
    path = "/api/v1/subscription",
    request_body = CancelDto,
    responses(
        (status = 200, description = "Subscription cancelled"),
        (status = 401, description = "Missing or invalid JWT",      body = ErrorResponse),
    ),
    tag = "Subscription",
    security(("bearer_token" = []))
)]
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

/// List subscription invoices for the current agency
#[utoipa::path(
    get,
    path = "/api/v1/subscription/invoices",
    params(PaginationQuery),
    responses(
        (status = 200, description = "Invoice list (opaque JSON array)"),
        (status = 401, description = "Missing or invalid JWT",      body = ErrorResponse),
    ),
    tag = "Subscription",
    security(("bearer_token" = []))
)]
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
// Platform-admin handlers
// ═════════════════════════════════════════════════════════════════════════════

/// [Admin] Get subscription status for any agency
#[utoipa::path(
    get,
    path = "/api/v1/admin/subscriptions/{agencyId}",
    params(("agencyId" = Uuid, Path, description = "Agency UUID")),
    responses(
        (status = 200, description = "Subscription state (opaque JSON object)"),
        (status = 401, description = "Missing or invalid admin token", body = ErrorResponse),
        (status = 404, description = "Agency not found",               body = ErrorResponse),
    ),
    tag = "Admin"
)]
pub async fn admin_get_status(
    State(state): State<AppState>,
    Path(agency_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let state_val = state.subscription.get_state.execute(agency_id).await?;
    Ok(Json(state_val))
}

/// [Admin] Get entitlements for any agency
#[utoipa::path(
    get,
    path = "/api/v1/admin/subscriptions/{agencyId}/entitlements",
    params(("agencyId" = Uuid, Path, description = "Agency UUID")),
    responses(
        (status = 200, description = "Entitlements map (opaque JSON object)"),
        (status = 401, description = "Missing or invalid admin token", body = ErrorResponse),
    ),
    tag = "Admin"
)]
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

/// [Admin] Change plan for any agency (supports custom pricing)
#[utoipa::path(
    patch,
    path = "/api/v1/admin/subscriptions/{agencyId}/plan",
    params(("agencyId" = Uuid, Path, description = "Agency UUID")),
    request_body = ChangePlanDto,
    responses(
        (status = 204, description = "Plan changed"),
        (status = 401, description = "Missing or invalid admin token", body = ErrorResponse),
    ),
    tag = "Admin"
)]
pub async fn admin_change_plan(
    State(state): State<AppState>,
    Path(agency_id): Path<Uuid>,
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

/// [Admin] Set a feature flag override for any agency
#[utoipa::path(
    post,
    path = "/api/v1/admin/subscriptions/{agencyId}/overrides",
    params(("agencyId" = Uuid, Path, description = "Agency UUID")),
    request_body = SetOverrideDto,
    responses(
        (status = 201, description = "Override set"),
        (status = 401, description = "Missing or invalid admin token", body = ErrorResponse),
    ),
    tag = "Admin"
)]
pub async fn admin_set_override(
    State(state): State<AppState>,
    Path(agency_id): Path<Uuid>,
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
            None,
            dto.expires_at,
        )
        .await?;
    Ok(StatusCode::CREATED)
}

/// [Admin] Remove a feature flag override for any agency
#[utoipa::path(
    delete,
    path = "/api/v1/admin/subscriptions/{agencyId}/overrides/{featureKey}",
    params(
        ("agencyId"   = Uuid,   Path, description = "Agency UUID"),
        ("featureKey" = String, Path, description = "Feature flag key"),
    ),
    responses(
        (status = 204, description = "Override removed"),
        (status = 401, description = "Missing or invalid admin token", body = ErrorResponse),
    ),
    tag = "Admin"
)]
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

/// [Admin] Cancel the subscription for any agency
#[utoipa::path(
    delete,
    path = "/api/v1/admin/subscriptions/{agencyId}",
    params(("agencyId" = Uuid, Path, description = "Agency UUID")),
    request_body = CancelDto,
    responses(
        (status = 200, description = "Subscription cancelled"),
        (status = 401, description = "Missing or invalid admin token", body = ErrorResponse),
    ),
    tag = "Admin"
)]
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
