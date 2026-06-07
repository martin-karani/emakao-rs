// ── REFACTORED ────────────────────────────────────────────────────────────────
// Changes from original:
//   - Added InvoiceLineItemResponse to own the nested line item domain type
//   - Replaced Vec<InvoiceLineItem> with Vec<InvoiceLineItemResponse> in InvoiceResponse
//   - Added rfc3339 serialization to all OffsetDateTime fields
//   - Removed created_by (internal audit detail)
//   - Applied #[serde(skip_serializing_if)] to optional fields
// ─────────────────────────────────────────────────────────────────────────────

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
pub struct InvoiceLineItemResponse {
    pub description: String,
    pub quantity: Decimal,
    pub unit_price_kes: Decimal,
    pub total_kes: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_type: Option<String>,
}

impl From<InvoiceLineItem> for InvoiceLineItemResponse {
    fn from(l: InvoiceLineItem) -> Self {
        Self {
            description: l.description,
            quantity: l.quantity,
            unit_price_kes: l.unit_price_kes,
            total_kes: l.total_kes,
            line_type: l.line_type,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InvoiceResponse {
    pub id: Uuid,
    pub agency_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<Uuid>,
    pub property_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agreement_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resident_id: Option<Uuid>,
    pub invoice_number: String,
    pub status: InvoiceStatus,
    pub line_items: Vec<InvoiceLineItemResponse>,
    pub subtotal_kes: Decimal,
    pub tax_kes: Decimal,
    pub total_kes: Decimal,
    pub due_date: Date,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(with = "time::serde::rfc3339::option", skip_serializing_if = "Option::is_none")]
    pub voided_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

// created_by intentionally excluded — internal audit detail.
// Use GET /api/v1/audit-log?entity=invoice&id={id} for actor history.
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
            line_items: i.line_items.into_iter().map(InvoiceLineItemResponse::from).collect(),
            subtotal_kes: i.subtotal_kes,
            tax_kes: i.tax.total_tax_kes,
            total_kes: i.total_kes,
            due_date: i.due_date,
            notes: i.notes,
            voided_at: i.voided_at,
            created_at: i.created_at,
            updated_at: i.updated_at,
        }
    }
}
