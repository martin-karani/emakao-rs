use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::agreement_repository::AgreementRepository},
    domain::agreement::Agreement,
};

pub struct ListAgreementsUseCase {
    pub repo: Arc<dyn AgreementRepository>,
}

impl ListAgreementsUseCase {
    pub fn new(repo: Arc<dyn AgreementRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        agency_id: Uuid,
        property_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Agreement>, AppError> {
        self.repo
            .find_all(agency_id, property_id, limit, offset)
            .await
    }
}
