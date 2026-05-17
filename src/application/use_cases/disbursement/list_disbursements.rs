use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::disbursement_repository::{DisbursementFilter, DisbursementRepository},
    },
    domain::{disbursement::Disbursement, enums::DisbursementStatus},
};

pub struct ListDisbursementsInput {
    pub agency_id: Uuid,
    pub owner_id: Option<Uuid>,
    pub property_id: Option<Uuid>,
    pub status: Option<DisbursementStatus>,
    pub limit: i64,
    pub offset: i64,
}

pub struct ListDisbursementsUseCase {
    repo: Arc<dyn DisbursementRepository>,
}

impl ListDisbursementsUseCase {
    pub fn new(repo: Arc<dyn DisbursementRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        input: ListDisbursementsInput,
    ) -> Result<Vec<Disbursement>, AppError> {
        self.repo
            .find_all(
                input.agency_id,
                DisbursementFilter {
                    owner_id: input.owner_id,
                    property_id: input.property_id,
                    status: input.status,
                    limit: input.limit.min(100),
                    offset: input.offset,
                },
            )
            .await
    }
}
