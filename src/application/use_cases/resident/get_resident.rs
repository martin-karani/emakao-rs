use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::resident_repository::ResidentRepository},
    domain::resident::Resident,
};

pub struct GetResidentUseCase {
    pub repo: Arc<dyn ResidentRepository>,
}

impl GetResidentUseCase {
    pub fn new(repo: Arc<dyn ResidentRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, agency_id: Uuid, id: Uuid) -> Result<Resident, AppError> {
        // Uses: ResidentRepository::find_by_id
        self.repo
            .find_by_id(agency_id, id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("resident {id}")))
    }
}