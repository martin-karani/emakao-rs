use std::sync::Arc;

use time::Date;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::accounting_repository::AccountingRepository},
    domain::accounting::{VatReport, VatReportFilter},
};

pub struct GetVatReportInput {
    pub agency_id: Uuid,
    pub period_start: Date,
    pub period_end: Date,
}

pub struct GetVatReportUseCase {
    pub repo: Arc<dyn AccountingRepository>,
}

impl GetVatReportUseCase {
    pub async fn execute(&self, input: GetVatReportInput) -> Result<VatReport, AppError> {
        if input.period_end < input.period_start {
            return Err(AppError::Validation(
                "period_end must be on or after period_start".into(),
            ));
        }

        self.repo
            .get_vat_report(VatReportFilter {
                agency_id: input.agency_id,
                period_start: input.period_start,
                period_end: input.period_end,
            })
            .await
    }
}
