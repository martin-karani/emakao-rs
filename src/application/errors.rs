use thiserror::Error;

use crate::domain::errors::DomainError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Domain(#[from] DomainError),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("external service error: {0}")]
    ExternalService(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("unauthorised")]
    Unauthorised,

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("plan upgrade required")]
    PlanUpgradeRequired,

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("unprocessable: {0}")]
    Unprocessable(String),

    #[error("internal server error: {0}")]
    InternalServer(String),

    // ── NEW: customisation layer ──────────────────────────────────────────────
    /// Returned by `require_feature!` when the agency's plan does not include
    /// the requested feature key.
    #[error("feature not available on this plan: {0}")]
    FeatureNotAvailable(String),

    /// Returned by `require_below_limit!` when a numeric plan limit is reached.
    /// First field = feature key, second = the limit value.
    #[error("plan limit reached for '{0}' (max {1})")]
    PlanLimitExceeded(String, i64),
}

impl From<garde::Report> for AppError {
    fn from(r: garde::Report) -> Self {
        let messages: Vec<String> = r
            .iter()
            .map(|(path, error)| format!("{}: {}", path, error))
            .collect();
        Self::Validation(messages.join("; "))
    }
}
