// src/domain/property_billing.rs

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct PropertyBillingSettings {
    pub property_id: Uuid,
    pub currency_code: String,
    pub rent_due_day: i32,
    pub late_fee_type: String, // "flat" | "percent"
    pub late_fee_value: Decimal,
    pub late_fee_grace_days: i32,

    pub water_rate_per_unit: Decimal,
    pub garbage_fee_kes: Decimal,
    pub security_fee_kes: Decimal,

    pub other_fixed_fees: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct PropertyBillingSummary {
    pub property_id: Uuid,
    pub property_name: String,
    pub currency_code: String,
    pub rent_due_day: i32,
    pub late_fee_type: String,
    pub late_fee_value: Decimal,
    pub late_fee_grace_days: i32,
    pub water_rate_per_unit: Decimal,
    pub garbage_fee_kes: Decimal,
    pub security_fee_kes: Decimal,
}
