use rust_decimal::Decimal;
use serde::Serialize;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::domain::enums::LedgerEntryType;

#[derive(Clone, Debug, Serialize)]
pub struct LedgerEntry {
    pub id: Uuid,
    pub agreement_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub owner_id: Option<Uuid>,
    pub entry_type: LedgerEntryType,
    pub amount_kes: Decimal,
    pub description: String,
    pub external_ref: Option<String>,
    pub mpesa_receipt: Option<String>,
    pub period_start: Option<Date>,
    pub period_end: Option<Date>,
    pub posted_by: Uuid,
    pub posted_at: OffsetDateTime,
    pub is_reconciled: bool,
    pub metadata: serde_json::Value,
}

pub struct CreateLedgerEntryCommand {
    pub agreement_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub owner_id: Option<Uuid>,
    pub entry_type: LedgerEntryType,
    pub amount_kes: Decimal,
    pub description: String,
    pub external_ref: Option<String>,
    pub mpesa_receipt: Option<String>,
    pub period_start: Option<Date>,
    pub period_end: Option<Date>,
    pub posted_by: Uuid,
    pub metadata: serde_json::Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct BalanceSummary {
    pub agreement_id: Uuid,
    pub total_charged: Decimal,
    pub total_paid: Decimal,
    pub outstanding: Decimal,
    pub last_payment_at: Option<OffsetDateTime>,
}
