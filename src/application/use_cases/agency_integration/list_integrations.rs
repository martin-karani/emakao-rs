// src/application/use_cases/agency_integration/list_integrations.rs

use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{
        errors::AppError, ports::agency_integration_repository::AgencyIntegrationRepository,
    },
    domain::agency_integration::AgencyIntegration,
};

pub struct ListIntegrationsUseCase {
    repo: Arc<dyn AgencyIntegrationRepository>,
}

impl ListIntegrationsUseCase {
    pub fn new(repo: Arc<dyn AgencyIntegrationRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, agency_id: Uuid) -> Result<Vec<AgencyIntegration>, AppError> {
        self.repo.list(agency_id).await
    }
}
