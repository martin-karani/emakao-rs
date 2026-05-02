use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("property name must not be empty")]
    PropertyNameEmpty,

    #[error("property limit exceeded – upgrade your plan")]
    PropertyLimitExceeded,

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("agreement {0} is not active")]
    AgreementNotActive(Uuid),

    #[error("unit {0} already has an active agreement")]
    UnitOccupied(Uuid),

    #[error("insufficient balance")]
    InsufficientBalance,
}
