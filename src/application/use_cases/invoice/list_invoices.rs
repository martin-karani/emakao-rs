// src/application/use_cases/invoice/list_invoices.rs

use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::invoice_repository::{InvoiceFilter, InvoiceRepository},
    },
    domain::{enums::InvoiceStatus, invoice::Invoice},
};

pub struct ListInvoicesUseCase {
    pub repo: Arc<dyn InvoiceRepository>,
}

pub struct ListInvoicesInput {
    pub agency_id: Uuid,
    pub property_id: Option<Uuid>,
    pub agreement_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub status: Option<InvoiceStatus>,
    pub limit: i64,
    pub offset: i64,
}

impl ListInvoicesUseCase {
    pub fn new(repo: Arc<dyn InvoiceRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: ListInvoicesInput) -> Result<Vec<Invoice>, AppError> {
        self.repo
            .find_all(
                input.agency_id,
                InvoiceFilter {
                    property_id: input.property_id,
                    agreement_id: input.agreement_id,
                    resident_id: input.resident_id,
                    status: input.status,
                    limit: input.limit.clamp(1, 100),
                    offset: input.offset.max(0),
                },
            )
            .await
    }
}
