use rust_decimal::Decimal;
use std::sync::Arc;
use time::Date;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::invoice_repository::{CreateInvoiceCommand, InvoiceRepository},
    },
    domain::invoice::{Invoice, InvoiceLineItem},
    infrastructure::jobs::workflow::WorkflowEngine,
};

pub struct CreateInvoiceUseCase {
    pub repo: Arc<dyn InvoiceRepository>,
    pub workflow: Arc<WorkflowEngine>,
}

pub struct CreateInvoiceInput {
    pub agency_id: Uuid,
    pub owner_id: Option<Uuid>,
    pub property_id: Uuid,
    pub agreement_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub line_items: Vec<InvoiceLineItem>,
    pub due_date: Date,
    pub notes: Option<String>,
    pub created_by: Uuid,
}

impl CreateInvoiceUseCase {
    pub fn new(repo: Arc<dyn InvoiceRepository>, workflow: Arc<WorkflowEngine>) -> Self {
        Self { repo, workflow }
    }

    pub async fn execute(&self, input: CreateInvoiceInput) -> Result<Invoice, AppError> {
        if input.line_items.is_empty() {
            return Err(AppError::Validation(
                "invoice must have at least one line item".into(),
            ));
        }

        // Validate each line item
        for item in &input.line_items {
            if item.description.trim().is_empty() {
                return Err(AppError::Validation(
                    "line item description must not be empty".into(),
                ));
            }
            if item.quantity <= Decimal::ZERO {
                return Err(AppError::Validation(
                    "line item quantity must be > 0".into(),
                ));
            }
            if item.unit_price_kes < Decimal::ZERO {
                return Err(AppError::Validation(
                    "line item unit_price_kes must be >= 0".into(),
                ));
            }
        }

        let invoice = self
            .repo
            .create(CreateInvoiceCommand {
                agency_id: input.agency_id,
                owner_id: input.owner_id,
                property_id: input.property_id,
                agreement_id: input.agreement_id,
                resident_id: input.resident_id,
                line_items: input.line_items,
                due_date: input.due_date,
                notes: input.notes,
                created_by: input.created_by,
            })
            .await?;

        tracing::info!(
            invoice_id = %invoice.id,
            invoice_number = %invoice.invoice_number,
            total_kes = %invoice.total_kes,
            "invoice created"
        );

        // ── Trigger Workflow ──────────────────────────────────────────────────
        let context = serde_json::json!({
            "invoice_id": invoice.id,
            "invoice_number": invoice.invoice_number,
            "total_kes": invoice.total_kes,
            "due_date": invoice.due_date,
            "resident_id": invoice.resident_id,
            "property_id": invoice.property_id,
        });

        let _ = self
            .workflow
            .trigger(
                input.agency_id,
                "invoice.created",
                "invoice",
                invoice.id,
                context,
            )
            .await;

        Ok(invoice)
    }
}
