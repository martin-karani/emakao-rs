// src/application/ports/accounting_repository.rs

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::accounting::{
        Account, AccountFilter, CreateAccountCommand, CreateJournalEntryCommand, JournalEntry,
        JournalEntryFilter, TrialBalance, VoidJournalEntryCommand,
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

    /// Deletes a non-system account.  Returns `Conflict` if the account has
    /// any journal lines referencing it.
    async fn delete_account(&self, agency_id: Uuid, id: Uuid) -> Result<(), AppError>;

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

    /// Inserts the entry header + lines and optionally posts it.
    /// When `post_immediately` is true the status is set to `posted` and
    /// account balances are updated atomically inside a transaction.
    async fn create_journal_entry(
        &self,
        cmd: CreateJournalEntryCommand,
    ) -> Result<JournalEntry, AppError>;

    async fn void_journal_entry(
        &self,
        cmd: VoidJournalEntryCommand,
    ) -> Result<JournalEntry, AppError>;

    // ── Trial balance ─────────────────────────────────────────────────────────

    async fn get_trial_balance(&self, agency_id: Uuid) -> Result<TrialBalance, AppError>;
}
