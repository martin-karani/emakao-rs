use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::ledger_repository::LedgerRepository},
    domain::ledger::BalanceSummary,
};

pub struct GetBalanceUseCase {
    pub repo: Arc<dyn LedgerRepository>,
}

impl GetBalanceUseCase {
    pub fn new(repo: Arc<dyn LedgerRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, agreement_id: Uuid) -> Result<BalanceSummary, AppError> {
        self.repo.balance_for_agreement(agreement_id).await
    }
}
