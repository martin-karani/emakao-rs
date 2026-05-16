use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::domain::enums::{AgreementStatus, BillingFrequency};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Agreement {
    pub id: Uuid,
    pub property_id: Uuid,
    pub unit_id: Uuid,
    pub resident_id: Uuid,
    pub start_date: Date,
    pub end_date: Option<Date>,
    pub rent_amount_kes: Decimal,
    pub deposit_kes: Decimal,
    pub billing_frequency: BillingFrequency,
    pub status: AgreementStatus,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
/// Minimal projection used by the billing scheduler — avoids loading
/// the full Agreement for bulk billing runs.
#[derive(Debug, Clone)]
pub struct ActiveAgreementBillingView {
    pub id: uuid::Uuid,
    pub unit_id: uuid::Uuid,
    pub resident_id: uuid::Uuid,
    pub property_id: uuid::Uuid,
    pub rent_amount_kes: rust_decimal::Decimal,
    pub billing_day: i32,          // day-of-month rent is due
    pub billing_frequency: String, // "monthly", "quarterly", etc.
    pub start_date: time::Date,
    pub end_date: Option<time::Date>,
}
