use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::agreement_repository::AgreementRepository},
    domain::agreement::Agreement,
};

pub struct GetAgreementUseCase {
    pub repo: Arc<dyn AgreementRepository>,
}

impl GetAgreementUseCase {
    pub fn new(repo: Arc<dyn AgreementRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, agency_id: Uuid, id: Uuid) -> Result<Agreement, AppError> {
        self.repo
            .find_by_id(agency_id, id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("agreement {id}")))
    }
}
