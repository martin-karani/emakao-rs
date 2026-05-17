use rust_decimal::Decimal;
use serde::Serialize;
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::bank_reconciliation::{
    BankStatement, BankStatementLine, ReconciliationReport, ReconciliationStatus,
};

// ── Statement line ────────────────────────────────────────────────────────────

/// A single transaction line from an imported bank statement.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BankStatementLineResponse {
    pub id: Uuid,
    pub statement_id: Uuid,
    pub value_date: Date,
    pub description: String,
    /// Positive = credit / money in.  Negative = debit / money out.
    pub amount: Decimal,
    pub reference: Option<String>,
    /// UUID of the matched journal entry, if reconciled.
    pub matched_entry_id: Option<Uuid>,
    pub status: ReconciliationStatus,
    pub created_at: OffsetDateTime,
}

impl From<BankStatementLine> for BankStatementLineResponse {
    fn from(l: BankStatementLine) -> Self {
        Self {
            id: l.id,
            statement_id: l.statement_id,
            value_date: l.value_date,
            description: l.description,
            amount: l.amount,
            reference: l.reference,
            matched_entry_id: l.matched_entry_id,
            status: l.status,
            created_at: l.created_at,
        }
    }
}

// ── Statement ─────────────────────────────────────────────────────────────────

/// A bank statement with its transaction lines.
///
/// The `lines` field is populated for single-statement GET requests.
/// List responses omit lines for performance — use the detail endpoint to
/// fetch a specific statement with its full set of lines.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BankStatementResponse {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub bank_name: String,
    pub account_number: String,
    pub statement_date: Date,
    pub opening_balance: Decimal,
    pub closing_balance: Decimal,
    /// Empty for list responses; populated by `GET /api/v1/bank-statements/:id`.
    pub lines: Vec<BankStatementLineResponse>,
    pub created_by: Uuid,
    pub created_at: OffsetDateTime,
}

impl From<BankStatement> for BankStatementResponse {
    fn from(s: BankStatement) -> Self {
        Self {
            id: s.id,
            agency_id: s.agency_id,
            bank_name: s.bank_name,
            account_number: s.account_number,
            statement_date: s.statement_date,
            opening_balance: s.opening_balance,
            closing_balance: s.closing_balance,
            lines: s
                .lines
                .into_iter()
                .map(BankStatementLineResponse::from)
                .collect(),
            created_by: s.created_by,
            created_at: s.created_at,
        }
    }
}

// ── Reconciliation report ─────────────────────────────────────────────────────

/// Reconciliation summary for a single bank statement.
///
/// `unmatched_lines` lists every line that has not yet been linked to a
/// journal entry — the primary work list for the bookkeeper.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReconciliationReportResponse {
    pub statement_id: Uuid,
    pub bank_name: String,
    pub account_number: String,
    pub statement_date: Date,
    pub opening_balance: Decimal,
    pub closing_balance: Decimal,
    /// Sum of all matched line amounts (positive = credits, negative = debits).
    pub matched_total: Decimal,
    /// Sum of all unmatched line amounts.
    pub unmatched_total: Decimal,
    pub matched_count: i64,
    pub unmatched_count: i64,
    /// Lines that still need a matching journal entry.
    pub unmatched_lines: Vec<BankStatementLineResponse>,
}

impl From<ReconciliationReport> for ReconciliationReportResponse {
    fn from(r: ReconciliationReport) -> Self {
        Self {
            statement_id: r.statement_id,
            bank_name: r.bank_name,
            account_number: r.account_number,
            statement_date: r.statement_date,
            opening_balance: r.opening_balance,
            closing_balance: r.closing_balance,
            matched_total: r.matched_total,
            unmatched_total: r.unmatched_total,
            matched_count: r.matched_count,
            unmatched_count: r.unmatched_count,
            unmatched_lines: r
                .unmatched_lines
                .into_iter()
                .map(BankStatementLineResponse::from)
                .collect(),
        }
    }
}
