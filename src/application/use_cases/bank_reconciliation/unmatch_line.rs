use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{
        errors::AppError, ports::bank_reconciliation_repository::BankReconciliationRepository,
    },
    domain::bank_reconciliation::{BankStatementLine, UnmatchLineCommand},
};

pub struct UnmatchLineInput {
    pub agency_id: Uuid,
    pub line_id: Uuid,
}

pub struct UnmatchLineUseCase {
    pub repo: Arc<dyn BankReconciliationRepository>,
}

impl UnmatchLineUseCase {
    pub async fn execute(&self, input: UnmatchLineInput) -> Result<BankStatementLine, AppError> {
        self.repo
            .unmatch_line(UnmatchLineCommand {
                agency_id: input.agency_id,
                line_id: input.line_id,
            })
            .await
    }
}
