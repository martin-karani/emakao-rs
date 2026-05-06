use std::sync::Arc;

use crate::{
    application::{errors::AppError, ports::property_repository::PropertyRepository},
    domain::property::{Property, UpdatePropertyCommand},
};

pub struct UpdatePropertyUseCase {
    pub repo: Arc<dyn PropertyRepository>,
}

impl UpdatePropertyUseCase {
    pub fn new(repo: Arc<dyn PropertyRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, cmd: UpdatePropertyCommand) -> Result<Property, AppError> {
        self.repo.update(cmd).await
    }
}
