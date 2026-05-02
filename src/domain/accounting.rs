use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalEntryStatus {
    Draft,
    Posted,
    Voided,
}

#[derive(Clone, Debug, Serialize)]
pub struct Account {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    pub balance: Decimal,
    pub is_system: bool,
    pub created_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize)]
pub struct JournalLine {
    pub account_id: Uuid,
    pub debit_kes: Decimal,
    pub credit_kes: Decimal,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct JournalEntry {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub reference: String,
    pub status: JournalEntryStatus,
    pub lines: Vec<JournalLine>,
    pub posted_by: Uuid,
    pub posted_at: OffsetDateTime,
}
