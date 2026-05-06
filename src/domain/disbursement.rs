use crate::domain::enums::{DisbursementMethod, DisbursementStatus};
use rust_decimal::Decimal;
use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct Disbursement {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub owner_id: Uuid,
    pub property_id: Uuid,
    pub amount_kes: Decimal,
    pub method: DisbursementMethod,
    pub reference: Option<String>,
    pub status: DisbursementStatus,
    pub period_start: time::Date,
    pub period_end: time::Date,
    pub notes: Option<String>,
    pub created_by: Uuid,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

pub struct CreateDisbursementCommand {
    pub agency_id: Uuid,
    pub owner_id: Uuid,
    pub property_id: Uuid,
    pub amount_kes: Decimal,
    pub method: DisbursementMethod,
    pub period_start: time::Date,
    pub period_end: time::Date,
    pub notes: Option<String>,
    pub created_by: Uuid,
}
