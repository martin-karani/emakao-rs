// src/application/use_cases/disbursement/initiate_payout.rs
//
// Moves a disbursement from Pending → Processing and, for M-Pesa B2C
// disbursements, fires an STK/B2C request via the M-Pesa adapter.
//
// The M-Pesa callback will later flip the status to Completed or Failed via
// the webhook handler — this use-case only kicks off the async flow.

use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::disbursement_repository::DisbursementRepository},
    domain::{
        disbursement::Disbursement,
        enums::{DisbursementMethod, DisbursementStatus},
    },
};

pub struct InitiatePayoutInput {
    pub agency_id: Uuid,
    pub disbursement_id: Uuid,
    pub initiated_by: Uuid,
}

pub struct InitiatePayoutUseCase {
    pub repo: Arc<dyn DisbursementRepository>,
    // Extend with Arc<dyn MpesaPort> when B2C is wired:
    // pub mpesa: Arc<dyn MpesaPort>,
}

impl InitiatePayoutUseCase {
    pub fn new(repo: Arc<dyn DisbursementRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: InitiatePayoutInput) -> Result<Disbursement, AppError> {
        let d = self
            .repo
            .find_by_id(input.agency_id, input.disbursement_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("disbursement {}", input.disbursement_id)))?;

        if d.status != DisbursementStatus::Pending {
            return Err(AppError::Validation(format!(
                "disbursement {} is not in Pending status — cannot initiate payout",
                input.disbursement_id
            )));
        }

        // For bank-transfer and cheque methods we just mark as Processing; the
        // finance team handles the actual transfer offline.
        // For MpesaB2c: fire B2C request here (TODO: wire MpesaPort).
        if d.method == DisbursementMethod::MpesaB2c {
            tracing::info!(
                disbursement_id = %d.id,
                "M-Pesa B2C payout initiation — adapter not yet wired; marking Processing"
            );
        }

        self.repo
            .update_status(
                input.agency_id,
                input.disbursement_id,
                DisbursementStatus::Processing,
                None,
            )
            .await
    }
}
