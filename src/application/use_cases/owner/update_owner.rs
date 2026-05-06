use std::sync::Arc;

use crate::{
    application::{errors::AppError, ports::owner_repository::OwnerRepository},
    domain::owner::{Owner, UpdateOwnerCommand},
};

pub struct UpdateOwnerUseCase {
    pub repo: Arc<dyn OwnerRepository>,
}

impl UpdateOwnerUseCase {
    pub fn new(repo: Arc<dyn OwnerRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, cmd: UpdateOwnerCommand) -> Result<Owner, AppError> {
        self.repo.update(cmd).await
    }
}
