use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::property_repository::PropertyRepository},
    domain::property::Property,
};

pub struct GetPropertyUseCase {
    pub repo: Arc<dyn PropertyRepository>,
}

impl GetPropertyUseCase {
    pub fn new(repo: Arc<dyn PropertyRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, agency_id: Uuid, id: Uuid) -> Result<Property, AppError> {
        self.repo
            .find_by_id(agency_id, id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("property {id}")))
    }
}
