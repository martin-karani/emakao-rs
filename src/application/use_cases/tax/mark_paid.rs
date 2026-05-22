// src/application/use_cases/tax/mark_paid.rs
//
// Marks a tax obligation as paid once the agency has used the KRA
// Payment Registration Number (PRN) to complete the payment on iTax
// or via mobile money / bank transfer.
//
// Flow:
//   1. Staff files the return on iTax → obligation status becomes `filed`.
//   2. Staff pays using the PRN → calls this use case with the PRN.
//   3. Obligation status becomes `paid`; `remitted_at` is stamped.

use std::sync::Arc;

use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::tax_repository::TaxRepository},
    domain::{
        errors::DomainError,
        tax::{MarkTaxPaidCommand, TaxObligation, TaxObligationStatus},
    },
};

// ── Use case ──────────────────────────────────────────────────────────────────

pub struct MarkPaidUseCase {
    pub repo: Arc<dyn TaxRepository>,
}

pub struct MarkPaidInput {
    pub agency_id: Uuid,
    pub obligation_id: Uuid,
    /// KRA Payment Registration Number received after filing on iTax / eRITS.
    pub kra_prn: String,
}

impl MarkPaidUseCase {
    pub fn new(repo: Arc<dyn TaxRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: MarkPaidInput) -> Result<TaxObligation, AppError> {
        // ── Load & ownership check ─────────────────────────────────────────
        let obligation = self
            .repo
            .find_obligation_by_id(input.agency_id, input.obligation_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("tax obligation {}", input.obligation_id)))?;

        // ── Guard: already paid ────────────────────────────────────────────
        if obligation.status == TaxObligationStatus::Paid {
            return Err(AppError::Domain(DomainError::InvalidInput(
                "obligation is already marked as paid".into(),
            )));
        }

        // ── Guard: must be filed before it can be paid ─────────────────────
        //
        // We allow `Overdue` → `Paid` directly so that late payments don't
        // get blocked (the overdue status is informational, not a hard lock).
        // `Pending` → `Paid` is also allowed for agencies that pay without
        // filing first (e.g. bulk PRN payments on iTax).
        if obligation.status == TaxObligationStatus::NilFiled {
            return Err(AppError::Domain(DomainError::InvalidInput(
                "nil-filed obligations have no payment due".into(),
            )));
        }

        // ── Validation ─────────────────────────────────────────────────────
        let prn = input.kra_prn.trim().to_string();
        if prn.is_empty() {
            return Err(AppError::Validation("kra_prn must not be empty".into()));
        }

        // ── Persist ────────────────────────────────────────────────────────
        let updated = self
            .repo
            .mark_paid(MarkTaxPaidCommand {
                obligation_id: input.obligation_id,
                kra_prn: prn.clone(),
                remitted_at: OffsetDateTime::now_utc(),
            })
            .await?;

        tracing::info!(
            obligation_id   = %updated.id,
            obligation_type = %updated.obligation_type,
            tax_period      = %updated.tax_period,
            tax_kes         = %updated.tax_kes,
            kra_prn         = %prn,
            "tax obligation marked paid"
        );

        Ok(updated)
    }
}
