use rust_decimal::Decimal;
use serde::Serialize;
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::{
    enums::InvoiceStatus,
    invoice::{Invoice, InvoiceLineItem},
};

#[derive(Debug, Serialize, ToSchema)]
pub struct InvoiceResponse {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub owner_id: Option<Uuid>,
    pub property_id: Uuid,
    pub agreement_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub invoice_number: String,
    pub status: InvoiceStatus,
    pub line_items: Vec<InvoiceLineItem>,
    pub subtotal_kes: Decimal,
    pub tax_kes: Decimal,
    pub total_kes: Decimal,
    pub due_date: Date,
    pub notes: Option<String>,
    pub voided_at: Option<OffsetDateTime>,
    pub created_by: Uuid,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<Invoice> for InvoiceResponse {
    fn from(i: Invoice) -> Self {
        Self {
            id: i.id,
            agency_id: i.agency_id,
            owner_id: i.owner_id,
            property_id: i.property_id,
            agreement_id: i.agreement_id,
            resident_id: i.resident_id,
            invoice_number: i.invoice_number,
            status: i.status,
            line_items: i.line_items,
            subtotal_kes: i.subtotal_kes,
            tax_kes: i.tax.total_tax_kes,
            total_kes: i.total_kes,
            due_date: i.due_date,
            notes: i.notes,
            voided_at: i.voided_at,
            created_by: i.created_by,
            created_at: i.created_at,
            updated_at: i.updated_at,
        }
    }
}
