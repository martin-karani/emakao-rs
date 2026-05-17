use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::bank_reconciliation::{
        BankStatement, BankStatementLine, ImportStatementCommand, MatchLineCommand,
        ReconciliationReport, UnmatchLineCommand,
    },
};

#[async_trait]
pub trait BankReconciliationRepository: Send + Sync + 'static {
    async fn list_statements(
        &self,
        agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<BankStatement>, AppError>;

    async fn find_statement_by_id(
        &self,
        agency_id: Uuid,
        id: Uuid,
    ) -> Result<Option<BankStatement>, AppError>;

    async fn import_statement(
        &self,
        cmd: ImportStatementCommand,
    ) -> Result<BankStatement, AppError>;

    async fn find_line_by_id(
        &self,
        agency_id: Uuid,
        line_id: Uuid,
    ) -> Result<Option<BankStatementLine>, AppError>;

    async fn match_line(&self, cmd: MatchLineCommand) -> Result<BankStatementLine, AppError>;

    async fn unmatch_line(&self, cmd: UnmatchLineCommand) -> Result<BankStatementLine, AppError>;

    async fn get_reconciliation_report(
        &self,
        agency_id: Uuid,
        statement_id: Uuid,
    ) -> Result<ReconciliationReport, AppError>;
}
