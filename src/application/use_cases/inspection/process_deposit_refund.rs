use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::{
            agreement_repository::AgreementRepository,
            inspection_repository::InspectionRepository,
            ledger_repository::LedgerRepository,
        },
    },
    domain::{
        enums::{InspectionStatus, InspectionType, LedgerEntryType},
        ledger::{CreateLedgerEntryCommand, LedgerEntry},
    },
};

// ── Input / Output types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundDeduction {
    pub description: String,
    pub amount_kes: Decimal,
}

pub struct ProcessDepositRefundInput {
    /// The move-out inspection that triggered this refund
    pub inspection_id: Uuid,
    /// Agreement associated with the tenancy
    pub agreement_id: Uuid,
    /// Line-item deductions (damages, cleaning, etc.)
    pub deductions: Vec<RefundDeduction>,
    /// Optional free-text notes to attach to the ledger entry
    pub notes: Option<String>,
    /// Staff member processing the refund
    pub posted_by: Uuid,
}

pub struct DepositRefundResult {
    pub inspection_id: Uuid,
    pub agreement_id: Uuid,
    /// Original deposit held
    pub deposit_kes: Decimal,
    /// Sum of all deductions
    pub deductions_total_kes: Decimal,
    /// Amount to be returned to tenant (deposit - deductions)
    pub refund_amount_kes: Decimal,
    /// Deduction line items used for calculation
    pub deductions: Vec<RefundDeduction>,
    /// The ledger entry created
    pub ledger_entry: LedgerEntry,
}

// ── Use case ──────────────────────────────────────────────────────────────────

pub struct ProcessDepositRefundUseCase {
    pub inspection_repo: Arc<dyn InspectionRepository>,
    pub agreement_repo: Arc<dyn AgreementRepository>,
    pub ledger_repo: Arc<dyn LedgerRepository>,
}

impl ProcessDepositRefundUseCase {
    pub fn new(
        inspection_repo: Arc<dyn InspectionRepository>,
        agreement_repo: Arc<dyn AgreementRepository>,
        ledger_repo: Arc<dyn LedgerRepository>,
    ) -> Self {
        Self {
            inspection_repo,
            agreement_repo,
            ledger_repo,
        }
    }

    pub async fn execute(
        &self,
        input: ProcessDepositRefundInput,
    ) -> Result<DepositRefundResult, AppError> {
        // ── 1. Validate the inspection ────────────────────────────────────────
        let inspection = self
            .inspection_repo
            .find_by_id(input.inspection_id)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("inspection {}", input.inspection_id))
            })?;

        if inspection.inspection_type != InspectionType::MoveOut {
            return Err(AppError::Validation(
                "deposit refund can only be processed after a move-out inspection".into(),
            ));
        }

        if inspection.status != InspectionStatus::Completed {
            return Err(AppError::Validation(
                "move-out inspection must be completed before processing a deposit refund".into(),
            ));
        }

        // Ensure the inspection is linked to the supplied agreement
        match inspection.agreement_id {
            Some(id) if id != input.agreement_id => {
                return Err(AppError::Validation(
                    "inspection is not linked to the supplied agreement".into(),
                ));
            }
            None => {
                return Err(AppError::Validation(
                    "inspection has no associated agreement — cannot process refund".into(),
                ));
            }
            _ => {} // id == input.agreement_id, OK
        }

        // ── 2. Fetch the original deposit amount ──────────────────────────────
        let deposit_kes = self
            .agreement_repo
            .get_deposit_kes(input.agreement_id)
            .await?;

        // ── 3. Validate deductions ────────────────────────────────────────────
        for deduction in &input.deductions {
            if deduction.amount_kes <= Decimal::ZERO {
                return Err(AppError::Validation(
                    "each deduction amount must be greater than zero".into(),
                ));
            }
            if deduction.description.trim().is_empty() {
                return Err(AppError::Validation(
                    "each deduction must have a description".into(),
                ));
            }
        }

        let deductions_total_kes: Decimal =
            input.deductions.iter().map(|d| d.amount_kes).sum();

        if deductions_total_kes > deposit_kes {
            return Err(AppError::Validation(format!(
                "total deductions ({deductions_total_kes} KES) exceed the security deposit ({deposit_kes} KES)"
            )));
        }

        let refund_amount_kes = deposit_kes - deductions_total_kes;

        // ── 4. Build metadata JSON (deduction breakdown) ──────────────────────
        let metadata = serde_json::json!({
            "inspection_id": input.inspection_id,
            "deposit_kes": deposit_kes,
            "deductions_total_kes": deductions_total_kes,
            "deductions": input.deductions,
            "notes": input.notes,
        });

        let description = match &input.notes {
            Some(n) => format!("Deposit refund — {n}"),
            None => "Security deposit refund".into(),
        };

        // ── 5. Post the deposit_refund ledger entry ───────────────────────────
        // Note: even if refund_amount_kes == 0 (full deduction of deposit) we
        // still post the entry to record the accounting closure.
        let ledger_entry = self
            .ledger_repo
            .create(CreateLedgerEntryCommand {
                agreement_id: Some(input.agreement_id),
                unit_id: Some(inspection.unit_id),
                resident_id: None,
                owner_id: None,
                entry_type: LedgerEntryType::DepositRefund,
                amount_kes: refund_amount_kes,
                description,
                external_ref: None,
                mpesa_receipt: None,
                period_start: None,
                period_end: None,
                posted_by: input.posted_by,
                metadata,
            })
            .await?;

        tracing::info!(
            inspection_id = %input.inspection_id,
            agreement_id  = %input.agreement_id,
            deposit_kes   = %deposit_kes,
            deductions    = %deductions_total_kes,
            refund        = %refund_amount_kes,
            "deposit refund processed"
        );

        Ok(DepositRefundResult {
            inspection_id: input.inspection_id,
            agreement_id: input.agreement_id,
            deposit_kes,
            deductions_total_kes,
            refund_amount_kes,
            deductions: input.deductions,
            ledger_entry,
        })
    }
}
