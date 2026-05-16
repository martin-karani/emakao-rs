use rust_decimal::Decimal;
use serde::Serialize;
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::enums::InvoiceStatus;

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct InvoiceLineItem {
    pub description: String,
    pub quantity: Decimal,
    pub unit_price_kes: Decimal,
    pub total_kes: Decimal,
}

#[derive(Clone, Debug, Serialize)]
pub struct Invoice {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub property_id: Uuid,
    pub agreement_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub invoice_number: String,
    pub line_items: Vec<InvoiceLineItem>,
    pub subtotal_kes: Decimal,
    pub tax_kes: Decimal,
    pub total_kes: Decimal,
    pub due_date: Date,
    pub status: InvoiceStatus,
    pub notes: Option<String>,
    pub created_by: Uuid,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub voided_at: Option<OffsetDateTime>,
}
