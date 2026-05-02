use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::resident_repository::ResidentRepository},
    domain::resident::Resident,
};

pub struct ListResidentsUseCase {
    pub repo: Arc<dyn ResidentRepository>,
}

impl ListResidentsUseCase {
    pub fn new(repo: Arc<dyn ResidentRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Resident>, AppError> {
        // Uses: ResidentRepository::find_all
        self.repo.find_all(agency_id, limit, offset).await
    }
}