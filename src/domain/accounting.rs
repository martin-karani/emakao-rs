use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

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
    /// Running balance maintained by the application when journal entries are posted.
    pub balance: Decimal,
    /// System accounts (e.g. cash, accounts receivable) cannot be deleted.
    pub is_system: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct JournalLine {
    pub account_id: Uuid,
    /// Positive amount in KES.  Exactly one of debit_kes / credit_kes is non-zero.
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

/// A single row in the trial balance report.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct TrialBalanceLine {
    pub account_id: Uuid,
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    pub total_debits: Decimal,
    pub total_credits: Decimal,
    /// Net balance: debits - credits (positive = debit balance).
    pub net_balance: Decimal,
}

/// Trial balance — aggregate of all posted journal lines grouped by account.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct TrialBalance {
    pub lines: Vec<TrialBalanceLine>,
    pub total_debits: Decimal,
    pub total_credits: Decimal,
    /// `true` when total_debits == total_credits (books balance).
    pub is_balanced: bool,
}

// ── Commands ──────────────────────────────────────────────────────────────────

pub struct CreateAccountCommand {
    pub agency_id: Uuid,
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    /// Set `true` only when seeding system accounts during agency provisioning.
    pub is_system: bool,
}

pub struct CreateJournalEntryCommand {
    pub agency_id: Uuid,
    pub reference: String,
    pub description: Option<String>,
    pub lines: Vec<JournalLine>,
    pub posted_by: Uuid,
    /// When `true`, the entry is immediately posted and account balances are
    /// updated.  When `false`, the entry is saved as a draft.
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
