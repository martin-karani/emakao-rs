use std::sync::Arc;
use time::Date;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::tax_repository::{TaxObligationFilter, TaxRepository},
    },
    domain::tax::{TaxObligation, TaxObligationStatus, TaxObligationType},
};

pub struct ListObligationsUseCase {
    pub repo: Arc<dyn TaxRepository>,
}

pub struct ListObligationsInput {
    pub agency_id: Uuid,
    pub owner_id: Option<Uuid>,
    pub property_id: Option<Uuid>,
    pub obligation_type: Option<TaxObligationType>,
    pub status: Option<TaxObligationStatus>,
    pub tax_period: Option<Date>,
    pub limit: i64,
    pub offset: i64,
}

impl ListObligationsUseCase {
    pub fn new(repo: Arc<dyn TaxRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        input: ListObligationsInput,
    ) -> Result<Vec<TaxObligation>, AppError> {
        self.repo
            .list_obligations(TaxObligationFilter {
                agency_id: input.agency_id,
                owner_id: input.owner_id,
                property_id: input.property_id,
                obligation_type: input.obligation_type,
                status: input.status,
                tax_period: input.tax_period,
                due_on_or_before: None,
                limit: input.limit.clamp(1, 200),
                offset: input.offset.max(0),
            })
            .await
    }
}
