use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{
        errors::AppError, ports::bank_reconciliation_repository::BankReconciliationRepository,
    },
    domain::bank_reconciliation::ReconciliationReport,
};

pub struct GetReconciliationReportUseCase {
    pub repo: Arc<dyn BankReconciliationRepository>,
}

impl GetReconciliationReportUseCase {
    pub async fn execute(
        &self,
        agency_id: Uuid,
        statement_id: Uuid,
    ) -> Result<ReconciliationReport, AppError> {
        self.repo
            .get_reconciliation_report(agency_id, statement_id)
            .await
    }
}
