use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::utility_repository::UtilityRepository},
    domain::utility::UtilityMeter,
};

pub struct GetMeterUseCase {
    pub repo: Arc<dyn UtilityRepository>,
}

impl GetMeterUseCase {
    pub fn new(repo: Arc<dyn UtilityRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, meter_id: Uuid) -> Result<UtilityMeter, AppError> {
        self.repo
            .find_meter_by_id(meter_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("meter {meter_id}")))
    }
}
