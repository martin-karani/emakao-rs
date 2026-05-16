use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::invoice_repository::InvoiceRepository},
    domain::{enums::InvoiceStatus, invoice::Invoice},
};

pub struct UpdateInvoiceStatusUseCase {
    pub repo: Arc<dyn InvoiceRepository>,
}

pub struct UpdateInvoiceStatusInput {
    pub agency_id: Uuid,
    pub id: Uuid,
    pub status: InvoiceStatus,
}

impl UpdateInvoiceStatusUseCase {
    pub fn new(repo: Arc<dyn InvoiceRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: UpdateInvoiceStatusInput) -> Result<Invoice, AppError> {
        // Load first to confirm it belongs to this agency
        let existing = self
            .repo
            .find_by_id(input.agency_id, input.id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("invoice {}", input.id)))?;

        // Guard against invalid transitions
        match (&existing.status, &input.status) {
            (InvoiceStatus::Void, _) => {
                return Err(AppError::Domain(
                    crate::domain::errors::DomainError::InvalidInput(
                        "cannot update a voided invoice".into(),
                    ),
                ))
            }
            (InvoiceStatus::Paid, InvoiceStatus::Draft | InvoiceStatus::Sent) => {
                return Err(AppError::Domain(
                    crate::domain::errors::DomainError::InvalidInput(
                        "cannot revert a paid invoice to draft/sent".into(),
                    ),
                ))
            }
            _ => {}
        }

        let invoice = if input.status == InvoiceStatus::Void {
            self.repo.void(input.id).await?
        } else {
            self.repo.update_status(input.id, input.status).await?
        };

        tracing::info!(
            invoice_id = %invoice.id,
            status     = ?invoice.status,
            "invoice status updated"
        );

        Ok(invoice)
    }
}
