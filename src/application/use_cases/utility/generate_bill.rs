use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::{
            billing_repository::BillingRepository,
            utility_repository::UtilityRepository,
        },
    },
    domain::{enums::UtilityBillStatus, utility::UtilityBill},
};

pub struct GenerateBillUseCase {
    pub repo: Arc<dyn UtilityRepository>,
    pub billing_repo: Arc<dyn BillingRepository>,
}

impl GenerateBillUseCase {
    pub fn new(
        repo: Arc<dyn UtilityRepository>,
        billing_repo: Arc<dyn BillingRepository>,
    ) -> Self {
        Self { repo, billing_repo }
    }

    /// Derives consumption from the two most recent readings, then writes a bill.
    pub async fn execute(&self, meter_id: Uuid) -> Result<UtilityBill, AppError> {
        // Uses: UtilityRepository::find_meter_by_id
        let meter = self
            .repo
            .find_meter_by_id(meter_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("meter {meter_id}")))?;

        // Uses: UtilityRepository::last_two_readings
        let readings = self.repo.last_two_readings(meter_id).await?;

        if readings.len() < 2 {
            return Err(AppError::Validation(
                "at least two readings are required to generate a bill".into(),
            ));
        }

        let (latest, previous) = (&readings[0], &readings[1]);
        let units_consumed = latest.reading_value - previous.reading_value;

        if units_consumed < rust_decimal::Decimal::ZERO {
            return Err(AppError::Validation(
                "latest reading is less than previous reading — check meter data".into(),
            ));
        }

        // 1. Try to get property-level rate override
        let mut rate = meter.rate_per_unit;
        if let Ok(Some(settings)) = self
            .billing_repo
            .get_billing_settings_by_property_id(meter.property_id)
            .await
        {
            if settings.water_rate_per_unit > rust_decimal::Decimal::ZERO
                && meter.meter_type == crate::domain::enums::MeterType::Water
            {
                rate = settings.water_rate_per_unit;
            }
        }

        let amount_kes = units_consumed * rate;

        let bill = UtilityBill {
            id: uuid::Uuid::new_v4(),
            meter_id,
            unit_id: meter.unit_id,
            units_consumed,
            amount_kes,
            status: UtilityBillStatus::Draft,
            created_at: time::OffsetDateTime::now_utc(),
        };

        // Uses: UtilityRepository::create_bill
        self.repo.create_bill(bill).await
    }
}
