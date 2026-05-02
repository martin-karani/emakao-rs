use crate::{
    application::errors::AppError,
    domain::subscription::{AgencyEntitlements, LimitCheckResult, LimitKey},
};

/// Quick boolean check (returns Err if over limit).
pub fn check_limit_allowed(
    entitlements: &AgencyEntitlements,
    key: &LimitKey,
    current: i32,
) -> Result<(), AppError> {
    if entitlements.within_limit(key, current) {
        Ok(())
    } else {
        let max = entitlements.get_limit(key);
        Err(AppError::Forbidden(format!(
            "You have reached the {} limit ({}) on your current plan. \
             Please upgrade to add more.",
            key.as_str(),
            max
        )))
    }
}

/// Full result with soft-limit flag (for usage dashboards).
pub fn check_limit_full(
    entitlements: &AgencyEntitlements,
    key: &LimitKey,
    current: i32,
    soft_limit: Option<i32>,
) -> LimitCheckResult {
    let max = entitlements.get_limit(key);
    LimitCheckResult {
        allowed: max == -1 || current < max,
        max,
        current,
        soft_limit_reached: entitlements.at_soft_limit(key, current, soft_limit),
    }
}
