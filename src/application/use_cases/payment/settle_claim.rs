use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::{
            ledger_repository::LedgerRepository,
            payment_repository::PaymentRepository,
            resident_repository::ResidentRepository,
        },
    },
    domain::{
        enums::{LedgerEntryType, PaymentClaimStatus},
        ledger::CreateLedgerEntryCommand,
    },
    infrastructure::providers::registry::ProviderRegistry,
};

pub struct SettlePaymentClaimUseCase {
    pub repo: Arc<dyn PaymentRepository>,
    pub ledger_repo: Arc<dyn LedgerRepository>,
    pub resident_repo: Arc<dyn ResidentRepository>,
    pub providers: Arc<ProviderRegistry>,
}

pub struct SettlePaymentClaimInput {
    pub agency_id: Uuid,
    pub claim_id: Uuid,
    pub mpesa_receipt: String,
    pub amount_kes: f64,
}

impl SettlePaymentClaimUseCase {
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

    pub async fn execute(&self, input: SettlePaymentClaimInput) -> Result<(), AppError> {
        let claim = self
            .repo
            .find_by_id(input.agency_id, input.claim_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("claim {}", input.claim_id)))?;

        // Allow settling if it's PendingReview (which we kept it as while waiting for Daraja)
        if claim.status != PaymentClaimStatus::PendingReview {
            tracing::warn!(claim_id = %input.claim_id, status = ?claim.status, "claim is not pending review, skipping settlement");
            return Ok(());
        }

        // Validate receipt matches
        if let Some(ref receipt) = claim.reference_code {
            if receipt.trim().to_uppercase() != input.mpesa_receipt.trim().to_uppercase() {
                tracing::warn!(
                    claim_id = %input.claim_id,
                    claim_receipt = %receipt,
                    webhook_receipt = %input.mpesa_receipt,
                    "Receipt mismatch during settlement"
                );
                return Err(AppError::Validation("M-Pesa receipt mismatch".into()));
            }
        }

        // Validate amount matches loosely (within 1 KES)
        let claim_amount: f64 = claim.amount_kes.try_into().unwrap_or(0.0);
        if (claim_amount - input.amount_kes).abs() > 1.0 {
            tracing::warn!(
                claim_id = %input.claim_id,
                claim_amount,
                webhook_amount = input.amount_kes,
                "Amount mismatch during settlement"
            );
            return Err(AppError::Validation("M-Pesa amount mismatch".into()));
        }

        let entry = self
            .ledger_repo
            .create(CreateLedgerEntryCommand {
                agreement_id: claim.agreement_id,
                unit_id: claim.unit_id,
                resident_id: claim.resident_id,
                owner_id: None,
                entry_type: LedgerEntryType::PaymentMpesa,
                amount_kes: claim.amount_kes,
                description: format!(
                    "Payment Reconciled: {}{}",
                    claim.payment_for.clone().unwrap_or_else(|| "rent".to_string()),
                    claim
                        .reference_code
                        .clone()
                        .map(|reference| format!(" ({reference})"))
                        .unwrap_or_default()
                ),
                external_ref: claim.reference_code.clone(),
                mpesa_receipt: claim.reference_code.clone(),
                period_start: None,
                period_end: None,
                posted_by: claim.submitted_by, // Or system ID
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

        // Send SMS
        if let Some(res_id) = claim.resident_id {
            if let Ok(Some(resident)) = self.resident_repo.find_by_id(input.agency_id, res_id).await {
                if let Some(phone) = resident.phone {
                    let sms_provider = self.providers.sms.for_agency(input.agency_id);
                    let body = format!(
                        "Confirmed: Your rent payment of KES {} has been received{}.",
                        claim.amount_kes,
                        if let Some(r) = &claim.reference_code { format!(" via receipt {}", r) } else { "".to_string() }
                    );
                    
                    if let Err(e) = sms_provider.send(&phone, &body, None).await {
                        tracing::error!(error = %e, phone = %phone, "Failed to send payment confirmation SMS");
                    }
                }
            }
        }

        // Mark claim as approved
        self.repo
            .update_status(
                claim.id,
                PaymentClaimStatus::Approved,
                claim.submitted_by,
                Some("Automatically approved via Daraja Transaction Status webhook".to_string()),
                None,
                Some(entry.id),
            )
            .await?;

        tracing::info!(claim_id = %claim.id, "claim automatically settled via webhook");
        Ok(())
    }
}
