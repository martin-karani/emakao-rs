use crate::{
    application::errors::AppError,
    domain::subscription::{AgencyEntitlements, FeatureKey},
};

pub fn check_feature(entitlements: &AgencyEntitlements, key: FeatureKey) -> Result<(), AppError> {
    if entitlements.has_feature(&key) {
        Ok(())
    } else {
        Err(AppError::Forbidden(format!(
            "Your current plan does not include the '{}' feature. \
             Please upgrade your subscription to access this.",
            key.as_str()
        )))
    }
}
