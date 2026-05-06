use crate::domain::enums::{BillingMode, MeterType, UtilityBillStatus};
use rust_decimal::Decimal;
use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct UtilityMeter {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub meter_type: MeterType,
    pub billing_mode: BillingMode,
    pub meter_number: String,
    pub rate_per_unit: Decimal,
    pub created_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize)]
pub struct MeterReading {
    pub id: Uuid,
    pub meter_id: Uuid,
    pub reading_value: Decimal,
    pub read_at: OffsetDateTime,
    pub recorded_by: Uuid,
}

#[derive(Clone, Debug, Serialize)]
pub struct UtilityBill {
    pub id: Uuid,
    pub meter_id: Uuid,
    pub unit_id: Uuid,
    pub units_consumed: Decimal,
    pub amount_kes: Decimal,
    pub status: UtilityBillStatus,
    pub created_at: OffsetDateTime,
}

/// Command passed to `UtilityRepository::create_meter`.
pub struct CreateMeterCommand {
    pub unit_id: Uuid,
    pub meter_type: MeterType,
    pub billing_mode: BillingMode,
    pub meter_number: String,
    pub rate_per_unit: Decimal,
}

/// Command passed to `UtilityRepository::create_reading`.
pub struct CreateMeterReadingCommand {
    pub meter_id: Uuid,
    pub reading_value: Decimal,
    pub recorded_by: Uuid,
}
