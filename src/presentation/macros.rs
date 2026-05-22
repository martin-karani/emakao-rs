// src/presentation/macros.rs
//
// Guard macros for customisation-layer feature and limit enforcement.
//
// These are thin wrappers around `CustomisationState::entitlements`.
// They early-return `Err(AppError::FeatureNotAvailable)` or
// `Err(AppError::PlanLimitExceeded)` from the calling async fn.
//
// ## Usage
//
// ```rust
// use crate::presentation::macros::{require_feature, require_below_limit};
//
// async fn create_property(
//     State(state): State<AppState>,
//     Extension(user): Extension<AuthenticatedUser>,
//     Json(cmd): Json<CreatePropertyCommand>,
// ) -> Result<impl IntoResponse, AppError> {
//
//     // Gate on a boolean feature
//     require_feature!(state, user.agency_id, "owner_portal");
//
//     // Gate on a numeric limit (current count must be < max)
//     let current = property_repo::count(&pool, user.agency_id).await?;
//     require_below_limit!(state, user.agency_id, "max_properties", current);
//
//     // ... rest of handler
// }
// ```
//
// Both macros require `state` to be an `AppState` (not `Arc<AppState>`) and
// `with_customisation()` to have been called at startup.

/// Early-return `Err(AppError::FeatureNotAvailable(...))` if the agency's
/// plan does not include the named boolean feature.
///
/// Arguments:
///   $state      — `AppState` in scope
///   $agency_id  — `uuid::Uuid`
///   $key        — string literal or `&str`
#[macro_export]
macro_rules! require_feature {
    ($state:expr, $agency_id:expr, $key:expr) => {
        if !$state
            .custom()
            .entitlements
            .is_enabled($agency_id, $key)
            .await
        {
            return Err($crate::application::errors::AppError::FeatureNotAvailable(
                $key.to_string(),
            ));
        }
    };
}

/// Early-return `Err(AppError::PlanLimitExceeded(...))` if `$current` is
/// greater than or equal to the plan's numeric limit for `$key`.
/// If the plan has no limit for `$key` (returns `None`), the check passes.
///
/// Arguments:
///   $state      — `AppState` in scope
///   $agency_id  — `uuid::Uuid`
///   $key        — string literal or `&str`
///   $current    — `i64` — how many the agency currently has
#[macro_export]
macro_rules! require_below_limit {
    ($state:expr, $agency_id:expr, $key:expr, $current:expr) => {
        if let Some(max) = $state
            .custom()
            .entitlements
            .numeric_limit($agency_id, $key)
            .await
        {
            if ($current as i64) >= max {
                return Err($crate::application::errors::AppError::PlanLimitExceeded(
                    $key.to_string(),
                    max,
                ));
            }
        }
    };
}

pub use require_below_limit;
pub use require_feature;
