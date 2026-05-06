use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::payment_repository::PaymentRepository},
    domain::{enums::PaymentClaimStatus, payment::PaymentClaim},
};

pub struct ApproveClaimUseCase {
    pub repo: Arc<dyn PaymentRepository>,
}

pub struct ApproveClaimInput {
    pub agency_id: Uuid,
    pub claim_id: Uuid,
    pub reviewed_by: Uuid,
    pub approve: bool,
    pub review_notes: Option<String>,
    pub rejection_reason: Option<String>,
}

impl ApproveClaimUseCase {
    pub fn new(repo: Arc<dyn PaymentRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: ApproveClaimInput) -> Result<PaymentClaim, AppError> {
        // Uses: PaymentRepository::find_by_id
        let claim = self
            .repo
            .find_by_id(input.agency_id, input.claim_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("claim {}", input.claim_id)))?;

        if claim.status != PaymentClaimStatus::PendingReview {
            return Err(AppError::Validation(
                "only pending claims can be reviewed".into(),
            ));
        }

        if !input.approve && input.rejection_reason.is_none() {
            return Err(AppError::Validation("rejection reason is required".into()));
        }

        let new_status = if input.approve {
            PaymentClaimStatus::Approved
        } else {
            PaymentClaimStatus::Rejected
        };

        // Uses: PaymentRepository::update_status
        let updated = self
            .repo
            .update_status(
                input.claim_id,
                new_status,
                input.reviewed_by,
                input.review_notes,
                input.rejection_reason,
            )
            .await?;

        tracing::info!(
            claim_id = %input.claim_id,
            status = ?updated.status,
            reviewed_by = %input.reviewed_by,
            "payment claim reviewed"
        );
        Ok(updated)
    }
}
