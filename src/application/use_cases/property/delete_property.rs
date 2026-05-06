use std::sync::Arc;
use uuid::Uuid;

use crate::application::{errors::AppError, ports::property_repository::PropertyRepository};

pub struct DeletePropertyUseCase {
    pub repo: Arc<dyn PropertyRepository>,
}

impl DeletePropertyUseCase {
    pub fn new(repo: Arc<dyn PropertyRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, agency_id: Uuid, id: Uuid) -> Result<(), AppError> {
        self.repo.delete(agency_id, id).await
    }
}
