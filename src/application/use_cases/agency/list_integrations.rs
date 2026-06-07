// src/application/use_cases/agency/list_integrations.rs

use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{
        errors::AppError, ports::agency_repository::AgencyRepository,
    },
    domain::agency::AgencyIntegration,
};

pub struct ListIntegrationsUseCase {
    repo: Arc<dyn AgencyRepository>,
}

impl ListIntegrationsUseCase {
    pub fn new(repo: Arc<dyn AgencyRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, agency_id: Uuid) -> Result<Vec<AgencyIntegration>, AppError> {
        self.repo.list_integrations(agency_id).await
    }
}
