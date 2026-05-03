use rust_decimal::Decimal;
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::utility::{
    BillingMode, MeterReading, MeterType, UtilityBill, UtilityBillStatus, UtilityMeter,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct UtilityMeterResponse {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub meter_type: MeterType,
    pub billing_mode: BillingMode,
    pub meter_number: String,
    pub rate_per_unit: Decimal,
    pub created_at: OffsetDateTime,
}

impl From<UtilityMeter> for UtilityMeterResponse {
    fn from(m: UtilityMeter) -> Self {
        Self {
            id: m.id,
            unit_id: m.unit_id,
            meter_type: m.meter_type,
            billing_mode: m.billing_mode,
            meter_number: m.meter_number,
            rate_per_unit: m.rate_per_unit,
            created_at: m.created_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MeterReadingResponse {
    pub id: Uuid,
    pub meter_id: Uuid,
    pub reading_value: Decimal,
    pub read_at: OffsetDateTime,
    pub recorded_by: Uuid,
}

impl From<MeterReading> for MeterReadingResponse {
    fn from(r: MeterReading) -> Self {
        Self {
            id: r.id,
            meter_id: r.meter_id,
            reading_value: r.reading_value,
            read_at: r.read_at,
            recorded_by: r.recorded_by,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UtilityBillResponse {
    pub id: Uuid,
    pub meter_id: Uuid,
    pub unit_id: Uuid,
    pub units_consumed: Decimal,
    pub amount_kes: Decimal,
    pub status: UtilityBillStatus,
    pub created_at: OffsetDateTime,
}

impl From<UtilityBill> for UtilityBillResponse {
    fn from(b: UtilityBill) -> Self {
        Self {
            id: b.id,
            meter_id: b.meter_id,
            unit_id: b.unit_id,
            units_consumed: b.units_consumed,
            amount_kes: b.amount_kes,
            status: b.status,
            created_at: b.created_at,
        }
    }
}
