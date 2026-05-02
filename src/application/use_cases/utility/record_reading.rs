use std::sync::Arc;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::utility_repository::UtilityRepository,
    },
    domain::utility::{CreateMeterReadingCommand, MeterReading},
};

pub struct RecordReadingUseCase {
    pub repo: Arc<dyn UtilityRepository>,
}

pub struct RecordReadingInput {
    pub meter_id: Uuid,
    pub reading_value: Decimal,
    pub recorded_by: Uuid,
}

impl RecordReadingUseCase {
    pub fn new(repo: Arc<dyn UtilityRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: RecordReadingInput) -> Result<MeterReading, AppError> {
        if input.reading_value < Decimal::ZERO {
            return Err(AppError::Validation("reading value must be non-negative".into()));
        }

        // Uses: UtilityRepository::find_meter_by_id — guard existence
        self.repo
            .find_meter_by_id(input.meter_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("meter {}", input.meter_id)))?;

        // Uses: UtilityRepository::create_reading
        self.repo.create_reading(CreateMeterReadingCommand {
            meter_id: input.meter_id,
            reading_value: input.reading_value,
            recorded_by: input.recorded_by,
        }).await
    }
}