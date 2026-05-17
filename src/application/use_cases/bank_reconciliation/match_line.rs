use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{
        errors::AppError, ports::bank_reconciliation_repository::BankReconciliationRepository,
    },
    domain::bank_reconciliation::{BankStatementLine, MatchLineCommand},
};

pub struct MatchLineInput {
    pub agency_id: Uuid,
    pub line_id: Uuid,
    pub journal_entry_id: Uuid,
}

pub struct MatchLineUseCase {
    pub repo: Arc<dyn BankReconciliationRepository>,
}

impl MatchLineUseCase {
    pub async fn execute(&self, input: MatchLineInput) -> Result<BankStatementLine, AppError> {
        // Check the line exists and belongs to this agency.
        let line = self
            .repo
            .find_line_by_id(input.agency_id, input.line_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("statement line {}", input.line_id)))?;

        if line.matched_entry_id.is_some() {
            return Err(AppError::Conflict(
                "line is already matched to a journal entry; unmatch it first".into(),
            ));
        }

        self.repo
            .match_line(MatchLineCommand {
                agency_id: input.agency_id,
                line_id: input.line_id,
                journal_entry_id: input.journal_entry_id,
            })
            .await
    }
}
