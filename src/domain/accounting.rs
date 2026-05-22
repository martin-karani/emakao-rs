// ─────────────────────────────────────────────────────────────────────────────
// src/domain/accounting.rs  — PATCH (add to end of system_accounts() vec)
//
// These five new SystemAccountSeed entries must be appended to the existing
// `system_accounts()` function.  The rest of the file is unchanged.
//
// NEW ACCOUNTS:
//   2040  MRI Tax Payable          Liability — 7.5 % MRI collected, not yet remitted
//   2050  WHT Payable to KRA       Liability — 5 % WHT deducted from agency fee
//   2060  VAT Control              Liability — net VAT balance (output − input)
//   1050  WHT Receivable           Asset — 5 % WHT certificate receivable from owner
//   5100  MRI Expense (owner)      Expense — owner's MRI when agency is NOT WHT agent
// ─────────────────────────────────────────────────────────────────────────────

// Append these entries inside system_accounts() after the "5090 Other Expenses" entry:
//
//     // ── Tax control accounts (Kenya-specific) ────────────────────────────
//     SystemAccountSeed {
//         code: "2040",
//         name: "MRI Tax Payable",
//         account_type: AccountType::Liability,
//         vat_applicable: false,
//         vat_rate: None,
//     },
//     SystemAccountSeed {
//         code: "2050",
//         name: "Withholding Tax Payable — KRA",
//         account_type: AccountType::Liability,
//         vat_applicable: false,
//         vat_rate: None,
//     },
//     SystemAccountSeed {
//         code: "2060",
//         name: "VAT Control Account",
//         account_type: AccountType::Liability,
//         vat_applicable: false,
//         vat_rate: None,
//     },
//     SystemAccountSeed {
//         code: "1050",
//         name: "WHT Receivable (WHT Certificate)",
//         account_type: AccountType::Asset,
//         vat_applicable: false,
//         vat_rate: None,
//     },
//     SystemAccountSeed {
//         code: "5100",
//         name: "MRI Tax Expense",
//         account_type: AccountType::Expense,
//         vat_applicable: false,
//         vat_rate: None,
//     },

// ─────────────────────────────────────────────────────────────────────────────
// FULL UPDATED FILE BELOW — complete replacement for src/domain/accounting.rs
// ─────────────────────────────────────────────────────────────────────────────

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

// ── Enums ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
}

impl AccountType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Asset => "asset",
            Self::Liability => "liability",
            Self::Equity => "equity",
            Self::Revenue => "revenue",
            Self::Expense => "expense",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "asset" => Some(Self::Asset),
            "liability" => Some(Self::Liability),
            "equity" => Some(Self::Equity),
            "revenue" => Some(Self::Revenue),
            "expense" => Some(Self::Expense),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum JournalEntryStatus {
    Draft,
    Posted,
    Voided,
}

impl JournalEntryStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Posted => "posted",
            Self::Voided => "voided",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(Self::Draft),
            "posted" => Some(Self::Posted),
            "voided" => Some(Self::Voided),
            _ => None,
        }
    }
}

// ── Entities ──────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct Account {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    pub balance: Decimal,
    pub is_system: bool,
    pub vat_applicable: bool,
    pub vat_rate: Option<Decimal>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct JournalLine {
    pub account_id: Uuid,
    pub debit_kes: Decimal,
    pub credit_kes: Decimal,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct JournalEntry {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub reference: String,
    pub description: Option<String>,
    pub status: JournalEntryStatus,
    pub lines: Vec<JournalLine>,
    pub posted_by: Uuid,
    pub posted_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
}

// ── Trial balance ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct TrialBalanceLine {
    pub account_id: Uuid,
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    pub total_debits: Decimal,
    pub total_credits: Decimal,
    pub net_balance: Decimal,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct TrialBalance {
    pub lines: Vec<TrialBalanceLine>,
    pub total_debits: Decimal,
    pub total_credits: Decimal,
    pub is_balanced: bool,
}

// ── VAT report ────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct VatReportLine {
    pub account_id: Uuid,
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    pub net_amount_kes: Decimal,
    pub vat_amount_kes: Decimal,
    pub vat_rate: Decimal,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct VatReport {
    pub period_start: Date,
    pub period_end: Date,
    pub output_vat_kes: Decimal,
    pub input_vat_kes: Decimal,
    pub net_vat_payable_kes: Decimal,
    pub lines: Vec<VatReportLine>,
}

// ── Commands ──────────────────────────────────────────────────────────────────

pub struct CreateAccountCommand {
    pub agency_id: Uuid,
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    pub is_system: bool,
    pub vat_applicable: bool,
    pub vat_rate: Option<Decimal>,
}

pub struct CreateJournalEntryCommand {
    pub agency_id: Uuid,
    pub reference: String,
    pub description: Option<String>,
    pub lines: Vec<JournalLine>,
    pub posted_by: Uuid,
    pub post_immediately: bool,
}

pub struct VoidJournalEntryCommand {
    pub agency_id: Uuid,
    pub entry_id: Uuid,
    pub voided_by: Uuid,
}

pub struct AccountFilter {
    pub agency_id: Uuid,
    pub account_type: Option<AccountType>,
    pub limit: i64,
    pub offset: i64,
}

pub struct JournalEntryFilter {
    pub agency_id: Uuid,
    pub status: Option<JournalEntryStatus>,
    pub limit: i64,
    pub offset: i64,
}

pub struct VatReportFilter {
    pub agency_id: Uuid,
    pub period_start: Date,
    pub period_end: Date,
}

// ── System chart of accounts ──────────────────────────────────────────────────

pub struct SystemAccountSeed {
    pub code: &'static str,
    pub name: &'static str,
    pub account_type: AccountType,
    pub vat_applicable: bool,
    pub vat_rate: Option<Decimal>,
}

pub fn system_accounts() -> Vec<SystemAccountSeed> {
    let vat16 = Some(Decimal::new(1600, 4)); // 0.1600 = 16 %

    vec![
        // ── Assets ────────────────────────────────────────────────────────
        SystemAccountSeed {
            code: "1010",
            name: "Cash on Hand",
            account_type: AccountType::Asset,
            vat_applicable: false,
            vat_rate: None,
        },
        SystemAccountSeed {
            code: "1020",
            name: "Bank Account",
            account_type: AccountType::Asset,
            vat_applicable: false,
            vat_rate: None,
        },
        SystemAccountSeed {
            code: "1030",
            name: "Accounts Receivable — Rent",
            account_type: AccountType::Asset,
            vat_applicable: false,
            vat_rate: None,
        },
        SystemAccountSeed {
            code: "1040",
            name: "Security Deposits Held",
            account_type: AccountType::Asset,
            vat_applicable: false,
            vat_rate: None,
        },
        // NEW: WHT certificate receivable (5 % deducted by owner on management fee)
        SystemAccountSeed {
            code: "1050",
            name: "WHT Receivable (WHT Certificate)",
            account_type: AccountType::Asset,
            vat_applicable: false,
            vat_rate: None,
        },
        // ── Liabilities ───────────────────────────────────────────────────
        SystemAccountSeed {
            code: "2010",
            name: "Accounts Payable",
            account_type: AccountType::Liability,
            vat_applicable: false,
            vat_rate: None,
        },
        SystemAccountSeed {
            code: "2020",
            name: "Tenant Deposits Payable",
            account_type: AccountType::Liability,
            vat_applicable: false,
            vat_rate: None,
        },
        SystemAccountSeed {
            code: "2030",
            name: "VAT Payable",
            account_type: AccountType::Liability,
            vat_applicable: false,
            vat_rate: None,
        },
        // NEW: MRI 7.5 % collected from tenants on behalf of owners, not yet remitted
        SystemAccountSeed {
            code: "2040",
            name: "MRI Tax Payable",
            account_type: AccountType::Liability,
            vat_applicable: false,
            vat_rate: None,
        },
        // NEW: 5 % WHT deducted on management fee, to be remitted to KRA
        SystemAccountSeed {
            code: "2050",
            name: "Withholding Tax Payable — KRA",
            account_type: AccountType::Liability,
            vat_applicable: false,
            vat_rate: None,
        },
        // NEW: net VAT control (output - input); clears to 2030 on filing
        SystemAccountSeed {
            code: "2060",
            name: "VAT Control Account",
            account_type: AccountType::Liability,
            vat_applicable: false,
            vat_rate: None,
        },
        // ── Equity ────────────────────────────────────────────────────────
        SystemAccountSeed {
            code: "3010",
            name: "Owner Equity",
            account_type: AccountType::Equity,
            vat_applicable: false,
            vat_rate: None,
        },
        SystemAccountSeed {
            code: "3020",
            name: "Retained Earnings",
            account_type: AccountType::Equity,
            vat_applicable: false,
            vat_rate: None,
        },
        // ── Revenue ───────────────────────────────────────────────────────
        SystemAccountSeed {
            code: "4010",
            name: "Rental Income",
            account_type: AccountType::Revenue,
            vat_applicable: true,
            vat_rate: vat16,
        },
        SystemAccountSeed {
            code: "4020",
            name: "Late Fee Income",
            account_type: AccountType::Revenue,
            vat_applicable: true,
            vat_rate: vat16,
        },
        SystemAccountSeed {
            code: "4030",
            name: "Service Charge Income",
            account_type: AccountType::Revenue,
            vat_applicable: true,
            vat_rate: vat16,
        },
        SystemAccountSeed {
            code: "4090",
            name: "Other Income",
            account_type: AccountType::Revenue,
            vat_applicable: false,
            vat_rate: None,
        },
        // ── Expenses ──────────────────────────────────────────────────────
        SystemAccountSeed {
            code: "5010",
            name: "Maintenance & Repairs",
            account_type: AccountType::Expense,
            vat_applicable: true,
            vat_rate: vat16,
        },
        SystemAccountSeed {
            code: "5020",
            name: "Utilities Expense",
            account_type: AccountType::Expense,
            vat_applicable: true,
            vat_rate: vat16,
        },
        SystemAccountSeed {
            code: "5030",
            name: "Management Fees",
            account_type: AccountType::Expense,
            vat_applicable: true,
            vat_rate: vat16,
        },
        SystemAccountSeed {
            code: "5040",
            name: "Bank Charges",
            account_type: AccountType::Expense,
            vat_applicable: false,
            vat_rate: None,
        },
        SystemAccountSeed {
            code: "5090",
            name: "Other Expenses",
            account_type: AccountType::Expense,
            vat_applicable: false,
            vat_rate: None,
        },
        // NEW: owner's MRI expense when agency is NOT acting as WHT agent
        SystemAccountSeed {
            code: "5100",
            name: "MRI Tax Expense",
            account_type: AccountType::Expense,
            vat_applicable: false,
            vat_rate: None,
        },
    ]
}
