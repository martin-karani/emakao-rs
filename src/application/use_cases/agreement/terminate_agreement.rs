use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::agreement_repository::AgreementRepository},
    domain::{enums::AgreementStatus, errors::DomainError},
};

pub struct TerminateAgreementUseCase {
    pub repo: Arc<dyn AgreementRepository>,
}

impl TerminateAgreementUseCase {
    pub fn new(repo: Arc<dyn AgreementRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        agency_id: Uuid,
        agreement_id: Uuid,
        terminated_by: Uuid,
    ) -> Result<(), AppError> {
        let agreement = self
            .repo
            .find_by_id(agency_id, agreement_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("agreement {agreement_id}")))?;

        if agreement.status != AgreementStatus::Active {
            return Err(DomainError::AgreementNotActive(agreement_id).into());
        }

        self.repo
            .update_status(agreement_id, AgreementStatus::Terminated, terminated_by)
            .await?;

        tracing::info!(
            agreement_id = %agreement_id,
            terminated_by = %terminated_by,
            "agreement terminated"
        );
        Ok(())
    }
}
