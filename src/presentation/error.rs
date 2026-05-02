//! The ONLY file in the codebase that maps AppError → HTTP responses.
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::{application::errors::AppError, domain::errors::DomainError};

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            // ── 400 ───────────────────────────────────────────────────────────
            AppError::Validation(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "VALIDATION_ERROR",
                msg.clone(),
            ),

            // ── 401 ───────────────────────────────────────────────────────────
            AppError::Unauthorised => (
                StatusCode::UNAUTHORIZED,
                "UNAUTHORISED",
                "authentication required".into(),
            ),

            // ── 402 ───────────────────────────────────────────────────────────
            AppError::PlanUpgradeRequired
            | AppError::Domain(DomainError::PropertyLimitExceeded) => (
                StatusCode::PAYMENT_REQUIRED,
                "PLAN_LIMIT_EXCEEDED",
                self.to_string(),
            ),

            // ── 403 ───────────────────────────────────────────────────────────
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", msg.clone()),

            // ── 404 ───────────────────────────────────────────────────────────
            AppError::NotFound(msg) | AppError::Domain(DomainError::NotFound(msg)) => {
                (StatusCode::NOT_FOUND, "NOT_FOUND", msg.clone())
            }

            // ── 422 domain rules ──────────────────────────────────────────────
            AppError::Domain(DomainError::PropertyNameEmpty)
            | AppError::Domain(DomainError::InvalidInput(_))
            | AppError::Domain(DomainError::AgreementNotActive(_))
            | AppError::Domain(DomainError::UnitOccupied(_))
            | AppError::Domain(DomainError::InsufficientBalance) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "DOMAIN_ERROR",
                self.to_string(),
            ),

            // ── 500 ───────────────────────────────────────────────────────────
            AppError::Database(e) => {
                tracing::error!(error = %e, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DATABASE_ERROR",
                    "a database error occurred".into(),
                )
            }
            AppError::ExternalService(msg) => {
                tracing::error!(error = %msg, "external service error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "EXTERNAL_SERVICE_ERROR",
                    "an external service error occurred".into(),
                )
            }
            _ => {
                tracing::error!(error = %self, "unhandled error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "an internal error occurred".into(),
                )
            }
        };

        (status, Json(json!({ "error": code, "message": message }))).into_response()
    }
}
