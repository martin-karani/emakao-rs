use std::sync::Arc;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::payment_repository::PaymentRepository,
    },
    domain::payment::{CreatePaymentClaimCommand, PaymentClaim, PaymentMethodType},
};

pub struct SubmitClaimUseCase {
    pub repo: Arc<dyn PaymentRepository>,
}

pub struct SubmitClaimInput {
    pub property_id: Uuid,
    pub agreement_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub submitted_by: Uuid,
    pub method_type: PaymentMethodType,
    pub amount_kes: Decimal,
    pub reference_code: Option<String>,
    pub proof_url: Option<String>,
    pub notes: Option<String>,
}

impl SubmitClaimUseCase {
    pub fn new(repo: Arc<dyn PaymentRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: SubmitClaimInput) -> Result<PaymentClaim, AppError> {
        if input.amount_kes <= Decimal::ZERO {
            return Err(AppError::Validation("payment amount must be positive".into()));
        }

        // Uses: PaymentRepository::create
        let claim = self.repo.create(CreatePaymentClaimCommand {
            property_id: input.property_id,
            agreement_id: input.agreement_id,
            resident_id: input.resident_id,
            submitted_by: input.submitted_by,
            method_type: input.method_type,
            amount_kes: input.amount_kes,
            reference_code: input.reference_code,
            proof_url: input.proof_url,
            notes: input.notes,
        }).await?;

        tracing::info!(claim_id = %claim.id, amount = %input.amount_kes, "payment claim submitted");
        Ok(claim)
    }
}