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
