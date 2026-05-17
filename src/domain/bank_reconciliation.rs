use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

// ── Enums ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationStatus {
    /// No matched journal entry — needs attention.
    Unmatched,
    /// Linked to a posted journal entry.
    Matched,
}

// ── Entities ──────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct BankStatementLine {
    pub id: Uuid,
    pub statement_id: Uuid,
    pub value_date: Date,
    pub description: String,
    /// Positive = credit (money in).  Negative = debit (money out).
    pub amount: Decimal,
    pub reference: Option<String>,
    pub matched_entry_id: Option<Uuid>,
    pub status: ReconciliationStatus,
    pub created_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct BankStatement {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub bank_name: String,
    pub account_number: String,
    pub statement_date: Date,
    pub opening_balance: Decimal,
    pub closing_balance: Decimal,
    pub lines: Vec<BankStatementLine>,
    pub created_by: Uuid,
    pub created_at: OffsetDateTime,
}

/// High-level reconciliation report for a statement.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ReconciliationReport {
    pub statement_id: Uuid,
    pub bank_name: String,
    pub account_number: String,
    pub statement_date: Date,
    pub opening_balance: Decimal,
    pub closing_balance: Decimal,
    /// Sum of all matched line amounts.
    pub matched_total: Decimal,
    /// Sum of all unmatched line amounts.
    pub unmatched_total: Decimal,
    pub matched_count: i64,
    pub unmatched_count: i64,
    pub unmatched_lines: Vec<BankStatementLine>,
}

// ── Commands ──────────────────────────────────────────────────────────────────

pub struct ImportStatementLineInput {
    pub value_date: Date,
    pub description: String,
    pub amount: Decimal,
    pub reference: Option<String>,
}

pub struct ImportStatementCommand {
    pub agency_id: Uuid,
    pub bank_name: String,
    pub account_number: String,
    pub statement_date: Date,
    pub opening_balance: Decimal,
    pub closing_balance: Decimal,
    pub lines: Vec<ImportStatementLineInput>,
    pub created_by: Uuid,
}

pub struct MatchLineCommand {
    pub agency_id: Uuid,
    pub line_id: Uuid,
    pub journal_entry_id: Uuid,
}

pub struct UnmatchLineCommand {
    pub agency_id: Uuid,
    pub line_id: Uuid,
}
