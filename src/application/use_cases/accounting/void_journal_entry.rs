use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::accounting_repository::AccountingRepository},
    domain::accounting::{JournalEntry, VoidJournalEntryCommand},
};

pub struct VoidJournalEntryInput {
    pub agency_id: Uuid,
    pub entry_id: Uuid,
    pub voided_by: Uuid,
}

pub struct VoidJournalEntryUseCase {
    pub repo: Arc<dyn AccountingRepository>,
}

impl VoidJournalEntryUseCase {
    pub async fn execute(&self, input: VoidJournalEntryInput) -> Result<JournalEntry, AppError> {
        let entry = self
            .repo
            .void_journal_entry(VoidJournalEntryCommand {
                agency_id: input.agency_id,
                entry_id: input.entry_id,
                voided_by: input.voided_by,
            })
            .await?;

        tracing::info!(
            entry_id  = %entry.id,
            reference = %entry.reference,
            voided_by = %input.voided_by,
            "journal entry voided"
        );

        Ok(entry)
    }
}
