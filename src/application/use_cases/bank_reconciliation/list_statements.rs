use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError, ports::bank_reconciliation_repository::BankReconciliationRepository,
    },
    domain::bank_reconciliation::BankStatement,
};

pub struct ListStatementsInput {
    pub agency_id: Uuid,
    pub limit: i64,
    pub offset: i64,
}

pub struct ListStatementsUseCase {
    pub repo: Arc<dyn BankReconciliationRepository>,
}

impl ListStatementsUseCase {
    pub async fn execute(&self, input: ListStatementsInput) -> Result<Vec<BankStatement>, AppError> {
        self.repo
            .list_statements(input.agency_id, input.limit, input.offset)
            .await
    }
}
