use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::disbursement_repository::DisbursementRepository},
    domain::{disbursement::Disbursement, enums::DisbursementStatus},
};

pub struct UpdateDisbursementStatusInput {
    pub agency_id: Uuid,
    pub id: Uuid,
    pub status: DisbursementStatus,
    /// Optional payment reference (e.g. M-Pesa transaction ID, bank ref).
    pub reference: Option<String>,
}

pub struct UpdateDisbursementStatusUseCase {
    repo: Arc<dyn DisbursementRepository>,
}

impl UpdateDisbursementStatusUseCase {
    pub fn new(repo: Arc<dyn DisbursementRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        input: UpdateDisbursementStatusInput,
    ) -> Result<Disbursement, AppError> {
        // Guard: can only move forward in the lifecycle
        let current = self
            .repo
            .find_by_id(input.agency_id, input.id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("disbursement {}", input.id)))?;

        use DisbursementStatus::*;
        let valid = matches!(
            (&current.status, &input.status),
            (Pending, Processing)
                | (Pending, Failed)
                | (Processing, Completed)
                | (Processing, Failed)
        );

        if !valid {
            return Err(AppError::Validation(format!(
                "cannot transition disbursement from {:?} to {:?}",
                current.status, input.status
            )));
        }

        self.repo
            .update_status(input.agency_id, input.id, input.status, input.reference)
            .await
    }
}
