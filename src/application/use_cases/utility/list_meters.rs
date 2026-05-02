use crate::{
    application::{errors::AppError, ports::utility_repository::UtilityRepository},
    domain::utility::UtilityMeter,
};
use std::sync::Arc;
use uuid::Uuid;

pub struct ListMetersUseCase {
    pub repo: Arc<dyn UtilityRepository>,
}

impl ListMetersUseCase {
    pub fn new(repo: Arc<dyn UtilityRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, unit_id: Uuid) -> Result<Vec<UtilityMeter>, AppError> {
        self.repo.find_meters_by_unit(unit_id).await
    }
}
