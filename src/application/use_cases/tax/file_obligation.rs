// src/application/use_cases/tax/file_obligation.rs
//
// Called from the API handler when staff clicks "File with KRA" in the UI,
// or automatically by the tax worker on the due date.
//
// Phase 1: stores a manual ack_number (staff enters it from iTax).
// Phase 2: will call GavaConnectClient::file_mri_return() and use the
//           ack from eRITS automatically.

use std::sync::Arc;

use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::tax_repository::TaxRepository},
    domain::{
        errors::DomainError,
        tax::{MarkTaxFiledCommand, TaxObligation, TaxObligationStatus},
    },
};

pub struct FileObligationUseCase {
    pub repo: Arc<dyn TaxRepository>,
}

pub struct FileObligationInput {
    pub agency_id: Uuid,
    pub obligation_id: Uuid,
    /// KRA acknowledgement number from eRITS / iTax.
    pub kra_ack_number: String,
}

impl FileObligationUseCase {
    pub fn new(repo: Arc<dyn TaxRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: FileObligationInput) -> Result<TaxObligation, AppError> {
        // Guard: obligation must exist for this agency
        let obligation = self
            .repo
            .find_obligation_by_id(input.agency_id, input.obligation_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("tax obligation {}", input.obligation_id)))?;

        // Cannot re-file a paid obligation
        if obligation.status == TaxObligationStatus::Paid {
            return Err(AppError::Domain(DomainError::InvalidInput(
                "cannot file an already-paid obligation".into(),
            )));
        }

        if input.kra_ack_number.trim().is_empty() {
            return Err(AppError::Validation(
                "kra_ack_number must not be empty".into(),
            ));
        }

        let updated = self
            .repo
            .mark_filed(MarkTaxFiledCommand {
                obligation_id: input.obligation_id,
                kra_ack_number: input.kra_ack_number,
                filed_at: OffsetDateTime::now_utc(),
            })
            .await?;

        tracing::info!(
            obligation_id  = %updated.id,
            obligation_type = %updated.obligation_type,
            period         = %updated.tax_period,
            tax_kes        = %updated.tax_kes,
            "tax obligation filed with KRA"
        );

        Ok(updated)
    }
}
