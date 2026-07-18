use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::{
            ledger_repository::LedgerRepository, payment_repository::PaymentRepository,
            resident_repository::ResidentRepository,
        },
    },
    domain::{
        enums::{LedgerEntryType, PaymentClaimStatus, PaymentMethodType},
        ledger::CreateLedgerEntryCommand,
        payment::PaymentClaim,
    },
    infrastructure::providers::registry::ProviderRegistry,
};

pub struct ApproveClaimUseCase {
    pub repo: Arc<dyn PaymentRepository>,
    pub ledger_repo: Arc<dyn LedgerRepository>,
    pub resident_repo: Arc<dyn ResidentRepository>,
    pub providers: Arc<ProviderRegistry>,
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
    pub fn new(
        repo: Arc<dyn PaymentRepository>,
        ledger_repo: Arc<dyn LedgerRepository>,
        resident_repo: Arc<dyn ResidentRepository>,
        providers: Arc<ProviderRegistry>,
    ) -> Self {
        Self {
            repo,
            ledger_repo,
            resident_repo,
            providers,
        }
    }

    pub async fn execute(&self, input: ApproveClaimInput) -> Result<PaymentClaim, AppError> {
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

        let mut ledger_entry_id = None;

        if input.approve {
            // 1. Verify with M-Pesa if it's an M-Pesa payment
            if let (
                PaymentMethodType::MpesaPaybill | PaymentMethodType::MpesaTill,
                Some(ref receipt),
            ) = (&claim.method_type, &claim.reference_code)
            {
                if let Some(mpesa_provider) = self.providers.payment.get(&input.agency_id) {
                    let is_valid = mpesa_provider
                        .verify_receipt(receipt, input.claim_id, input.agency_id)
                        .await
                        .unwrap_or(false);
                    if !is_valid {
                        return Err(AppError::Validation(format!(
                            "M-Pesa Transaction Status query failed to initiate for receipt {}",
                            receipt
                        )));
                    }

                    // The actual verification happens asynchronously via webhook.
                    // We keep the status as PendingReview until the webhook arrives.
                    let updated = self
                        .repo
                        .update_status(
                            input.claim_id,
                            PaymentClaimStatus::PendingReview,
                            input.reviewed_by,
                            Some(
                                "Verification Pending (Transaction Status Query Initiated)"
                                    .to_string(),
                            ),
                            None,
                            None,
                        )
                        .await?;

                    tracing::info!(claim_id = %input.claim_id, "Transaction Status Query initiated. Waiting for webhook.");
                    return Ok(updated);
                } else {
                    tracing::warn!(agency = %input.agency_id, "No M-Pesa provider configured, skipping verification");
                }
            }

            // If it's Cash (or MpesaProvider is missing), we do the synchronous ledger update
            let entry_type = match claim.method_type {
                PaymentMethodType::Cash => LedgerEntryType::PaymentCash,
                PaymentMethodType::BankTransfer => LedgerEntryType::PaymentBank,
                _ => LedgerEntryType::PaymentMpesa,
            };

            let entry = self
                .ledger_repo
                .create(CreateLedgerEntryCommand {
                    agreement_id: claim.agreement_id,
                    unit_id: claim.unit_id,
                    resident_id: claim.resident_id,
                    owner_id: None,
                    entry_type,
                    amount_kes: claim.amount_kes,
                    description: format!(
                        "Payment Reconciled: {}{}",
                        claim
                            .payment_for
                            .clone()
                            .unwrap_or_else(|| "rent".to_string()),
                        claim
                            .reference_code
                            .clone()
                            .map(|reference| format!(" ({reference})"))
                            .unwrap_or_default()
                    ),
                    external_ref: claim.reference_code.clone(),
                    mpesa_receipt: if entry_type == LedgerEntryType::PaymentMpesa {
                        claim.reference_code.clone()
                    } else {
                        None
                    },
                    period_start: None,
                    period_end: None,
                    posted_by: input.reviewed_by,
                    metadata: serde_json::json!({
                        "claim_id": claim.id,
                        "submitted_via": claim.submitted_via,
                        "payment_for": claim.payment_for,
                        "period_label": claim.period_label,
                        "allocation": claim.allocation,
                        "raw_message": claim.raw_message,
                    }),
                })
                .await?;

            ledger_entry_id = Some(entry.id);

            // Send SMS if resident exists
            if let Some(res_id) = claim.resident_id {
                if let Ok(Some(resident)) =
                    self.resident_repo.find_by_id(input.agency_id, res_id).await
                {
                    if let Some(phone) = resident.phone {
                        let sms_provider = self.providers.sms.for_agency(input.agency_id);
                        let body = format!(
                            "Confirmed: Your rent payment of KES {} has been received{}.",
                            claim.amount_kes,
                            if let Some(r) = &claim.reference_code {
                                format!(" via receipt {}", r)
                            } else {
                                "".to_string()
                            }
                        );

                        if let Err(e) = sms_provider.send(&phone, &body, None).await {
                            tracing::error!(error = %e, phone = %phone, "Failed to send payment confirmation SMS");
                        }
                    } else {
                        tracing::warn!(resident_id = %res_id, "Resident has no phone number, skipping SMS");
                    }
                }
            }
        }

        let new_status = if input.approve {
            PaymentClaimStatus::Approved
        } else {
            PaymentClaimStatus::Rejected
        };

        // 4. Update Claim Status
        let updated = self
            .repo
            .update_status(
                input.claim_id,
                new_status,
                input.reviewed_by,
                input.review_notes,
                input.rejection_reason,
                ledger_entry_id,
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
