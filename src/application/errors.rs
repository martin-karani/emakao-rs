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

    #[error("forbidden")]
    Forbidden(String),

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("plan upgrade required")]
    PlanUpgradeRequired,

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("unprocessable: {0}")]
    Unprocessable(String),

    #[error("internal server")]
    InternalServer(String),
}

impl From<garde::Report> for AppError {
    fn from(r: garde::Report) -> Self {
        // Collect all validation messages into one readable string
        let messages: Vec<String> = r
            .iter()
            .map(|(path, error)| format!("{}: {}", path, error))
            .collect();
        Self::Validation(messages.join("; "))
    }
}
