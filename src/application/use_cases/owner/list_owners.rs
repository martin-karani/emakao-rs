use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::owner_repository::OwnerRepository},
    domain::owner::Owner,
};

pub struct ListOwnersUseCase {
    pub repo: Arc<dyn OwnerRepository>,
}

impl ListOwnersUseCase {
    pub fn new(repo: Arc<dyn OwnerRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Owner>, AppError> {
        // Uses: OwnerRepository::find_all
        self.repo.find_all(agency_id, limit, offset).await
    }
}