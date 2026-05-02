use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerEntryType {
    Rent,
    Deposit,
    HoaDues,
    CamCharge,
    Utility,
    MaintenanceCharge,
    LateFee,
    LegalFee,
    Penalty,
    PaymentMpesa,
    PaymentBank,
    PaymentCash,
    DepositRefund,
    CreditNote,
    Waiver,
    Disbursement,
    JournalAdjustment,
}

impl LedgerEntryType {
    pub fn is_payment(&self) -> bool {
        matches!(
            self,
            Self::PaymentMpesa | Self::PaymentBank | Self::PaymentCash
        )
    }

    pub fn from_method_str(method_type: &str) -> Self {
        match method_type {
            "mpesa_paybill" | "mpesa_till" => Self::PaymentMpesa,
            "bank_transfer" => Self::PaymentBank,
            _ => Self::PaymentCash,
        }
    }
}

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

/// Command passed to `LedgerRepository::create`.
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
