use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::property_repository::{PropertyFilter, PropertyRepository},
    },
    domain::property::Property,
};

pub struct ListPropertiesUseCase {
    pub repo: Arc<dyn PropertyRepository>,
}

impl ListPropertiesUseCase {
    pub fn new(repo: Arc<dyn PropertyRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        agency_id: Uuid,
        property_type: Option<String>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Property>, AppError> {
        self.repo
            .find_all(PropertyFilter {
                agency_id,
                property_type,
                limit,
                offset,
            })
            .await
    }
}
