use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::property_repository::PropertyRepository},
    domain::{
        enums::PropertyType,
        errors::DomainError,
        property::{CreatePropertyCommand, Property, PropertyConfig},
    },
};

pub struct CreatePropertyUseCase {
    pub repo: Arc<dyn PropertyRepository>,
}

pub struct CreatePropertyInput {
    pub agency_id: Uuid,
    pub created_by: Uuid,
    pub name: String,
    pub address: String,
    pub city: String,
    pub property_type: PropertyType,
    pub config: PropertyConfig,
}

impl CreatePropertyUseCase {
    pub fn new(repo: Arc<dyn PropertyRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: CreatePropertyInput) -> Result<Property, AppError> {
        if input.name.trim().is_empty() {
            return Err(DomainError::PropertyNameEmpty.into());
        }

        let property = self
            .repo
            .create(CreatePropertyCommand {
                agency_id: input.agency_id,
                created_by: input.created_by,
                name: input.name,
                address: input.address,
                city: input.city,
                property_type: input.property_type,
                config: input.config,
            })
            .await?;

        tracing::info!(property_id = %property.id, "property created");
        Ok(property)
    }
}
