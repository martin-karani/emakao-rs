use rust_decimal::Decimal;
use serde::Serialize;
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::ledger::{BalanceSummary, LedgerEntry, LedgerEntryType};

#[derive(Debug, Serialize, ToSchema)]
pub struct LedgerEntryResponse {
    pub id: Uuid,
    pub agreement_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
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
}

impl From<LedgerEntry> for LedgerEntryResponse {
    fn from(e: LedgerEntry) -> Self {
        Self {
            id: e.id,
            agreement_id: e.agreement_id,
            unit_id: e.unit_id,
            resident_id: e.resident_id,
            entry_type: e.entry_type,
            amount_kes: e.amount_kes,
            description: e.description,
            external_ref: e.external_ref,
            mpesa_receipt: e.mpesa_receipt,
            period_start: e.period_start,
            period_end: e.period_end,
            posted_by: e.posted_by,
            posted_at: e.posted_at,
            is_reconciled: e.is_reconciled,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BalanceSummaryResponse {
    pub agreement_id: Uuid,
    pub total_charged: Decimal,
    pub total_paid: Decimal,
    pub outstanding: Decimal,
    pub last_payment_at: Option<OffsetDateTime>,
}

impl From<BalanceSummary> for BalanceSummaryResponse {
    fn from(b: BalanceSummary) -> Self {
        Self {
            agreement_id: b.agreement_id,
            total_charged: b.total_charged,
            total_paid: b.total_paid,
            outstanding: b.outstanding,
            last_payment_at: b.last_payment_at,
        }
    }
}
