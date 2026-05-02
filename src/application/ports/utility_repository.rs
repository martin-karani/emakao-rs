use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::utility::{
        CreateMeterCommand, CreateMeterReadingCommand, MeterReading, UtilityBill, UtilityMeter,
    },
};

#[async_trait]
pub trait UtilityRepository: Send + Sync + 'static {
    async fn find_meter_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<UtilityMeter>, AppError>;

    async fn create_meter(
        &self,
        cmd: CreateMeterCommand,
    ) -> Result<UtilityMeter, AppError>;

    async fn create_reading(
        &self,
        cmd: CreateMeterReadingCommand,
    ) -> Result<MeterReading, AppError>;

    async fn last_two_readings(
        &self,
        meter_id: Uuid,
    ) -> Result<Vec<MeterReading>, AppError>;

    async fn create_bill(
        &self,
        bill: UtilityBill,
    ) -> Result<UtilityBill, AppError>;
}