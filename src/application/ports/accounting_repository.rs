use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::accounting::{
        Account, AccountFilter, CreateAccountCommand, CreateJournalEntryCommand, JournalEntry,
        JournalEntryFilter, TrialBalance, VatReport, VatReportFilter, VoidJournalEntryCommand,
    },
};

#[async_trait]
pub trait AccountingRepository: Send + Sync + 'static {
    // ── Accounts ──────────────────────────────────────────────────────────────

    async fn list_accounts(&self, filter: AccountFilter) -> Result<Vec<Account>, AppError>;

    async fn find_account_by_id(
        &self,
        agency_id: Uuid,
        id: Uuid,
    ) -> Result<Option<Account>, AppError>;

    async fn create_account(&self, cmd: CreateAccountCommand) -> Result<Account, AppError>;

    async fn delete_account(&self, agency_id: Uuid, id: Uuid) -> Result<(), AppError>;

    /// Seeds the standard chart of accounts for a newly provisioned agency.
    /// All seeded accounts have `is_system = true`.
    /// Called once from `ProvisionAgencyUseCase` — not exposed via the API.
    async fn seed_system_accounts(&self, agency_id: Uuid) -> Result<(), AppError>;

    // ── Journal entries ───────────────────────────────────────────────────────

    async fn list_journal_entries(
        &self,
        filter: JournalEntryFilter,
    ) -> Result<Vec<JournalEntry>, AppError>;

    async fn find_journal_entry_by_id(
        &self,
        agency_id: Uuid,
        id: Uuid,
    ) -> Result<Option<JournalEntry>, AppError>;

    async fn create_journal_entry(
        &self,
        cmd: CreateJournalEntryCommand,
    ) -> Result<JournalEntry, AppError>;

    async fn void_journal_entry(
        &self,
        cmd: VoidJournalEntryCommand,
    ) -> Result<JournalEntry, AppError>;

    // ── Reports ───────────────────────────────────────────────────────────────

    async fn get_trial_balance(&self, agency_id: Uuid) -> Result<TrialBalance, AppError>;

    async fn get_vat_report(&self, filter: VatReportFilter) -> Result<VatReport, AppError>;
}
