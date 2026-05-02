use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::ledger_repository::LedgerRepository},
    domain::ledger::LedgerEntry,
};

pub struct ListLedgerEntriesUseCase {
    pub repo: Arc<dyn LedgerRepository>,
}

impl ListLedgerEntriesUseCase {
    pub fn new(repo: Arc<dyn LedgerRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        agreement_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<LedgerEntry>, AppError> {
        // Uses: LedgerRepository::find_by_agreement
        self.repo.find_by_agreement(agreement_id, limit, offset).await
    }
}