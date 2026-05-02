use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::owner_repository::OwnerRepository},
    domain::owner::Owner,
};

pub struct GetOwnerUseCase {
    pub repo: Arc<dyn OwnerRepository>,
}

impl GetOwnerUseCase {
    pub fn new(repo: Arc<dyn OwnerRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, agency_id: Uuid, id: Uuid) -> Result<Owner, AppError> {
        // Uses: OwnerRepository::find_by_id
        self.repo
            .find_by_id(agency_id, id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("owner {id}")))
    }
}