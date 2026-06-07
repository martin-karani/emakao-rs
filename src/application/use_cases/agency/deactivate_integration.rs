// src/application/use_cases/agency/deactivate_integration.rs
//
// Sets `is_active = false` for the given (provider_type, provider_key) pair.
//
// NOTE: evicting the entry from the live in-memory provider registry is an
// infrastructure orchestration concern and is handled by the caller (the HTTP
// handler) after this use case returns.

use std::sync::Arc;

use uuid::Uuid;

use crate::application::{
    errors::AppError, ports::agency_repository::AgencyRepository,
};

pub struct DeactivateIntegrationInput {
    pub agency_id: Uuid,
    pub provider_type: String,
    pub provider_key: String,
}

pub struct DeactivateIntegrationUseCase {
    repo: Arc<dyn AgencyRepository>,
}

impl DeactivateIntegrationUseCase {
    pub fn new(repo: Arc<dyn AgencyRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: DeactivateIntegrationInput) -> Result<(), AppError> {
        self.repo
            .deactivate_integration(input.agency_id, &input.provider_type, &input.provider_key)
            .await
    }
}
