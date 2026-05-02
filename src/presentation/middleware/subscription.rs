use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        use_cases::subscription::{
            check_feature::check_feature as uc_check_feature,
            check_limit::check_limit_allowed as uc_check_limit,
        },
    },
    domain::{
        agency::ResolvedAgency,
        subscription::{AgencyEntitlements, FeatureKey, LimitKey, SubscriptionState},
    },
    presentation::app_state::AppState,
};

/// Available in every handler after the subscription middleware runs.
#[derive(Clone)]
pub struct ResolvedSubscription {
    pub state: SubscriptionState,
    pub entitlements: AgencyEntitlements,
}

pub async fn subscription_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    // Extract agency_id — set by tenant_resolver middleware before this runs
    let agency_id: Option<Uuid> = request.extensions().get::<ResolvedAgency>().map(|a| a.id);

    // No agency context (platform-admin route) → pass through
    let Some(agency_id) = agency_id else {
        return next.run(request).await;
    };

    // ── Load state (cheap path: cache hit most of the time) ──────────────────
    let sub_state = match state.subscription.get_state.execute(agency_id).await {
        Ok(s) => s,
        Err(AppError::Forbidden(msg)) => return payment_required_response(&msg),
        Err(e) => {
            tracing::error!(%agency_id, error = %e, "subscription state check failed");
            return internal_error_response();
        }
    };

    // ── Check active ─────────────────────────────────────────────────────────
    if !sub_state.is_active {
        return if sub_state.is_trial_expired {
            payment_required_response(
                "Your 14-day trial has expired. Please upgrade your plan to continue using eMakao.",
            )
        } else if sub_state.status == "past_due" {
            payment_required_response(
                "Your subscription payment is past due. Please update your payment method.",
            )
        } else if sub_state.status == "cancelled" && sub_state.in_grace_period {
            // Within cancellation grace — still let them in (read-only ideally)
            payment_required_response(
                "Your subscription has been cancelled. You can still access your data until the end of the billing period.",
            )
        } else {
            payment_required_response("An active subscription is required to access this resource.")
        };
    }

    let entitlements = match state.subscription.get_entitlements.execute(agency_id).await {
        Ok(e) => e,
        Err(e) => {
            tracing::error!(%agency_id, error = %e, "entitlements load failed");
            return internal_error_response();
        }
    };

    request.extensions_mut().insert(ResolvedSubscription {
        state: sub_state,
        entitlements,
    });

    next.run(request).await
}

fn payment_required_response(message: &str) -> Response {
    (
        StatusCode::PAYMENT_REQUIRED, // 402
        Json(json!({ "error": message, "code": "subscription_inactive" })),
    )
        .into_response()
}

fn internal_error_response() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "Subscription check failed. Please try again." })),
    )
        .into_response()
}

/// Returns 403 if the resolved subscription does not include the feature.
///
/// ```rust
/// pub async fn my_handler(
///     Extension(sub): Extension<ResolvedSubscription>,
/// ) -> Result<impl IntoResponse, AppError> {
///     require_feature(&sub.entitlements, FeatureKey::CommWhatsapp)?;
///     // ...
/// }
/// ```
pub fn require_feature(entitlements: &AgencyEntitlements, key: FeatureKey) -> Result<(), AppError> {
    uc_check_feature(entitlements, key)
}

/// Returns 403 if `current` has reached or exceeded the plan limit.
pub fn require_limit(
    entitlements: &AgencyEntitlements,
    key: &LimitKey,
    current: i32,
) -> Result<(), AppError> {
    uc_check_limit(entitlements, key, current)
}
