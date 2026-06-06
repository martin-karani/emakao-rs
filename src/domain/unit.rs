use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;
use crate::domain::enums::UnitStatus;

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Unit {
    pub id: Uuid,
    pub property_id: Uuid,
    pub parent_unit_id: Option<Uuid>,
    pub unit_number: String,
    pub floor: Option<i32>,
    pub size_sqm: Option<Decimal>,
    pub bedrooms: Option<i16>,
    pub bathrooms: Option<i16>,
    pub rent_amount_kes: Decimal,
    pub deposit_kes: Decimal,
    pub status: UnitStatus,
    pub description: Option<String>,
    pub photos: serde_json::Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

pub struct CreateUnitCommand {
    pub property_id: Uuid,
    pub parent_unit_id: Option<Uuid>,
    pub unit_number: String,
    pub floor: Option<i32>,
    pub size_sqm: Option<Decimal>,
    pub bedrooms: Option<i16>,
    pub bathrooms: Option<i16>,
    pub rent_amount_kes: Decimal,
    pub deposit_kes: Decimal,
    pub description: Option<String>,
}

pub struct UpdateUnitCommand {
    pub id: Uuid,
    pub unit_number: Option<String>,
    pub floor: Option<Option<i32>>,
    pub size_sqm: Option<Option<Decimal>>,
    pub bedrooms: Option<Option<i16>>,
    pub bathrooms: Option<Option<i16>>,
    pub rent_amount_kes: Option<Decimal>,
    pub deposit_kes: Option<Decimal>,
    pub description: Option<Option<String>>,
    pub status: Option<UnitStatus>,
}
