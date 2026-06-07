// ── REFACTORED ────────────────────────────────────────────────────────────────
// Changes from original:
//   - Removed Clone from all response structs
//   - Added rfc3339 serialization to all OffsetDateTime fields
//   - Applied #[serde(skip_serializing_if)] to optional fields
//   - Kept posted_by with comment (needed for frontend audit trail display)
// ─────────────────────────────────────────────────────────────────────────────

use rust_decimal::Decimal;
use serde::Serialize;
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::accounting::{
    Account, AccountType, JournalEntry, JournalEntryStatus, JournalLine, TrialBalance,
    TrialBalanceLine, VatReport, VatReportLine,
};

// ── Account ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct AccountResponse {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    pub balance: Decimal,
    pub is_system: bool,
    pub vat_applicable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vat_rate: Option<Decimal>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl From<Account> for AccountResponse {
    fn from(a: Account) -> Self {
        Self {
            id: a.id,
            agency_id: a.agency_id,
            code: a.code,
            name: a.name,
            account_type: a.account_type,
            balance: a.balance,
            is_system: a.is_system,
            vat_applicable: a.vat_applicable,
            vat_rate: a.vat_rate,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}

// ── Journal entry ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct JournalLineResponse {
    pub account_id: Uuid,
    pub debit_kes: Decimal,
    pub credit_kes: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl From<JournalLine> for JournalLineResponse {
    fn from(l: JournalLine) -> Self {
        Self {
            account_id: l.account_id,
            debit_kes: l.debit_kes,
            credit_kes: l.credit_kes,
            description: l.description,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct JournalEntryResponse {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub reference: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: JournalEntryStatus,
    pub lines: Vec<JournalLineResponse>,
    // posted_by kept: needed by frontend to display financial audit trail
    pub posted_by: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    pub posted_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl From<JournalEntry> for JournalEntryResponse {
    fn from(e: JournalEntry) -> Self {
        Self {
            id: e.id,
            agency_id: e.agency_id,
            reference: e.reference,
            description: e.description,
            status: e.status,
            lines: e.lines.into_iter().map(JournalLineResponse::from).collect(),
            posted_by: e.posted_by,
            posted_at: e.posted_at,
            created_at: e.created_at,
        }
    }
}

// ── Trial balance ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct TrialBalanceLineResponse {
    pub account_id: Uuid,
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    pub total_debits: Decimal,
    pub total_credits: Decimal,
    /// Positive = debit balance; negative = credit balance.
    pub net_balance: Decimal,
}

impl From<TrialBalanceLine> for TrialBalanceLineResponse {
    fn from(l: TrialBalanceLine) -> Self {
        Self {
            account_id: l.account_id,
            code: l.code,
            name: l.name,
            account_type: l.account_type,
            total_debits: l.total_debits,
            total_credits: l.total_credits,
            net_balance: l.net_balance,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TrialBalanceResponse {
    pub lines: Vec<TrialBalanceLineResponse>,
    pub total_debits: Decimal,
    pub total_credits: Decimal,
    /// `true` when the books balance (total_debits == total_credits).
    pub is_balanced: bool,
}

impl From<TrialBalance> for TrialBalanceResponse {
    fn from(tb: TrialBalance) -> Self {
        Self {
            lines: tb
                .lines
                .into_iter()
                .map(TrialBalanceLineResponse::from)
                .collect(),
            total_debits: tb.total_debits,
            total_credits: tb.total_credits,
            is_balanced: tb.is_balanced,
        }
    }
}

// ── VAT report ────────────────────────────────────────────────────────────────

/// One account's contribution to the VAT return for the period.
#[derive(Debug, Serialize, ToSchema)]
pub struct VatReportLineResponse {
    pub account_id: Uuid,
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    /// Net turnover for the period, exclusive of VAT.
    pub net_amount_kes: Decimal,
    /// VAT at `vat_rate` applied to `net_amount_kes`.
    pub vat_amount_kes: Decimal,
    /// Fractional rate used, e.g. `0.1600` = 16 %.
    pub vat_rate: Decimal,
}

impl From<VatReportLine> for VatReportLineResponse {
    fn from(l: VatReportLine) -> Self {
        Self {
            account_id: l.account_id,
            code: l.code,
            name: l.name,
            account_type: l.account_type,
            net_amount_kes: l.net_amount_kes,
            vat_amount_kes: l.vat_amount_kes,
            vat_rate: l.vat_rate,
        }
    }
}

/// Aggregated VAT return for a calendar period.
///
/// `net_vat_payable_kes` = `output_vat_kes` − `input_vat_kes`.
/// A negative value means KRA owes a refund.
#[derive(Debug, Serialize, ToSchema)]
pub struct VatReportResponse {
    pub period_start: Date,
    pub period_end: Date,
    /// Output VAT — collected from tenants on VAT-applicable revenue accounts.
    pub output_vat_kes: Decimal,
    /// Input VAT — paid on VAT-applicable expense accounts.
    pub input_vat_kes: Decimal,
    /// Net amount payable to KRA.
    pub net_vat_payable_kes: Decimal,
    pub lines: Vec<VatReportLineResponse>,
}

impl From<VatReport> for VatReportResponse {
    fn from(r: VatReport) -> Self {
        Self {
            period_start: r.period_start,
            period_end: r.period_end,
            output_vat_kes: r.output_vat_kes,
            input_vat_kes: r.input_vat_kes,
            net_vat_payable_kes: r.net_vat_payable_kes,
            lines: r
                .lines
                .into_iter()
                .map(VatReportLineResponse::from)
                .collect(),
        }
    }
}
