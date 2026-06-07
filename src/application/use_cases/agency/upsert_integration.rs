// src/application/use_cases/agency/upsert_integration.rs
//
// Persists provider credentials (encrypted by the repository) and
// returns the redacted integration record.
//
// NOTE: reloading the live provider registry after a successful upsert is
// an infrastructure orchestration concern and is handled by the caller
// (the HTTP handler) after this use case returns.

use std::sync::Arc;

use uuid::Uuid;

use crate::application::{
    errors::AppError,
    ports::agency_repository::{AgencyRepository, UpsertIntegrationCommand},
};

pub struct UpsertIntegrationInput {
    pub agency_id: Uuid,
    pub provider_type: String,
    pub provider_key: String,
    /// Plain-text credentials — the repository encrypts before writing.
    pub credentials: serde_json::Value,
    pub settings: serde_json::Value,
}

pub struct UpsertIntegrationUseCase {
    repo: Arc<dyn AgencyRepository>,
}

impl UpsertIntegrationUseCase {
    pub fn new(repo: Arc<dyn AgencyRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: UpsertIntegrationInput) -> Result<(), AppError> {
        // Credentials must be a JSON object — a flat array or scalar is never valid.
        if !input.credentials.is_object() {
            return Err(AppError::Validation(
                "integration 'credentials' must be a JSON object".into(),
            ));
        }

        self.repo
            .upsert_integration(UpsertIntegrationCommand {
                agency_id: input.agency_id,
                provider_type: input.provider_type,
                provider_key: input.provider_key,
                credentials: input.credentials,
                settings: input.settings,
            })
            .await
    }
}
