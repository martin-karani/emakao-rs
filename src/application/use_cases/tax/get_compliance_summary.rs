use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::tax_repository::TaxRepository},
    domain::tax::{TaxComplianceSummary, TaxPeriod},
};

pub struct GetComplianceSummaryUseCase {
    pub repo: Arc<dyn TaxRepository>,
}

impl GetComplianceSummaryUseCase {
    pub fn new(repo: Arc<dyn TaxRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        agency_id: Uuid,
        tax_period: TaxPeriod,
    ) -> Result<TaxComplianceSummary, AppError> {
        self.repo
            .get_compliance_summary(agency_id, tax_period)
            .await
    }
}
