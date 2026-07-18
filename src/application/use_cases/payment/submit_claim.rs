use rust_decimal::Decimal;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::payment_repository::PaymentRepository},
    domain::{
        enums::PaymentMethodType,
        payment::{CreatePaymentClaimCommand, PaymentAllocationItem, PaymentClaim},
    },
};

pub struct SubmitClaimUseCase {
    pub repo: Arc<dyn PaymentRepository>,
}

pub struct SubmitClaimInput {
    pub property_id: Uuid,
    pub agreement_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub submitted_by: Uuid,
    pub method_type: PaymentMethodType,
    pub amount_kes: Decimal,
    pub reference_code: Option<String>,
    pub proof_url: Option<String>,
    pub notes: Option<String>,
    pub submitted_via: String,
    pub raw_message: Option<String>,
    pub period_label: Option<String>,
    pub payment_for: Option<String>,
    pub allocation: Vec<PaymentAllocationItem>,
}

impl SubmitClaimUseCase {
    pub fn new(repo: Arc<dyn PaymentRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: SubmitClaimInput) -> Result<PaymentClaim, AppError> {
        if input.amount_kes <= Decimal::ZERO {
            return Err(AppError::Validation(
                "payment amount must be positive".into(),
            ));
        }

        let claim = self
            .repo
            .create(CreatePaymentClaimCommand {
                property_id: input.property_id,
                agreement_id: input.agreement_id,
                resident_id: input.resident_id,
                unit_id: input.unit_id,
                submitted_by: input.submitted_by,
                method_type: input.method_type,
                amount_kes: input.amount_kes,
                reference_code: input.reference_code,
                proof_url: input.proof_url,
                notes: input.notes,
                submitted_via: input.submitted_via,
                raw_message: input.raw_message,
                period_label: input.period_label,
                payment_for: input.payment_for,
                allocation: input.allocation,
            })
            .await?;

        tracing::info!(claim_id = %claim.id, amount = %input.amount_kes, "payment claim submitted");
        Ok(claim)
    }
}
