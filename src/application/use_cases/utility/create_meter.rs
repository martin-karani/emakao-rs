use rust_decimal::Decimal;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::utility_repository::UtilityRepository},
    domain::{
        enums::{BillingMode, MeterType},
        utility::{CreateMeterCommand, UtilityMeter},
    },
};

pub struct CreateMeterUseCase {
    pub repo: Arc<dyn UtilityRepository>,
}

pub struct CreateMeterInput {
    pub unit_id: Uuid,
    pub meter_type: MeterType,
    pub billing_mode: BillingMode,
    pub meter_number: String,
    pub rate_per_unit: Decimal,
}

impl CreateMeterUseCase {
    pub fn new(repo: Arc<dyn UtilityRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: CreateMeterInput) -> Result<UtilityMeter, AppError> {
        if input.rate_per_unit <= Decimal::ZERO {
            return Err(AppError::Validation(
                "rate per unit must be positive".into(),
            ));
        }

        self.repo
            .create_meter(CreateMeterCommand {
                unit_id: input.unit_id,
                meter_type: input.meter_type,
                billing_mode: input.billing_mode,
                meter_number: input.meter_number,
                rate_per_unit: input.rate_per_unit,
            })
            .await
    }
}
