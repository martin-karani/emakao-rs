use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::disbursement_repository::DisbursementRepository},
    domain::disbursement::Disbursement,
};

pub struct GetDisbursementUseCase {
    repo: Arc<dyn DisbursementRepository>,
}

impl GetDisbursementUseCase {
    pub fn new(repo: Arc<dyn DisbursementRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, agency_id: Uuid, id: Uuid) -> Result<Disbursement, AppError> {
        self.repo
            .find_by_id(agency_id, id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("disbursement {id}")))
    }
}
