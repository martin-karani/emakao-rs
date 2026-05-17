use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::accounting_repository::AccountingRepository},
    domain::accounting::{JournalEntry, JournalEntryFilter, JournalEntryStatus},
};

pub struct ListJournalEntriesInput {
    pub agency_id: Uuid,
    pub status: Option<JournalEntryStatus>,
    pub limit: i64,
    pub offset: i64,
}

pub struct ListJournalEntriesUseCase {
    pub repo: Arc<dyn AccountingRepository>,
}

impl ListJournalEntriesUseCase {
    pub async fn execute(
        &self,
        input: ListJournalEntriesInput,
    ) -> Result<Vec<JournalEntry>, AppError> {
        self.repo
            .list_journal_entries(JournalEntryFilter {
                agency_id: input.agency_id,
                status: input.status,
                limit: input.limit.clamp(1, 200),
                offset: input.offset.max(0),
            })
            .await
    }
}
