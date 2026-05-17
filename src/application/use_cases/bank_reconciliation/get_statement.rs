use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError, ports::bank_reconciliation_repository::BankReconciliationRepository,
    },
    domain::bank_reconciliation::BankStatement,
};

pub struct GetStatementUseCase {
    pub repo: Arc<dyn BankReconciliationRepository>,
}

impl GetStatementUseCase {
    pub async fn execute(&self, agency_id: Uuid, id: Uuid) -> Result<BankStatement, AppError> {
        self.repo
            .find_statement_by_id(agency_id, id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("bank statement {id}")))
    }
}
