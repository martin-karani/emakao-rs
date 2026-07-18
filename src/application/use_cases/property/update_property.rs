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
        // Check for existing property with the same name if we're updating name
        if let Some(name) = &cmd.name {
            let trimmed_name = name.trim();
            let existing_property = self.repo.find_by_name(cmd.agency_id, trimmed_name).await?;
            if let Some(existing) = existing_property {
                if existing.id != cmd.id {
                    return Err(AppError::Validation("A property with this name already exists in your agency".into()));
                }
            }
        }
        
        self.repo.update(cmd).await
    }
}
