use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::{
    enums::InvoiceStatus,
    tax::{EtimsInvoiceRef, TaxBreakdown},
};

// ── Line item ─────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct InvoiceLineItem {
    pub description: String,
    pub quantity: Decimal,
    pub unit_price_kes: Decimal,
    pub total_kes: Decimal,
    /// e.g. "rent" | "management_fee" | "utility" | "deposit" | "late_fee"
    pub line_type: Option<String>,
}

// ── Invoice ───────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize)]
pub struct Invoice {
    pub id: Uuid,
    pub agency_id: Uuid,
    /// Owner the rent / management-fee flows to / from (used for MRI linkage).
    pub owner_id: Option<Uuid>,
    pub property_id: Uuid,
    pub agreement_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub invoice_number: String,
    pub line_items: Vec<InvoiceLineItem>,
    /// Sum of all line_item.total_kes (VAT-exclusive).
    pub subtotal_kes: Decimal,
    /// Full Kenya tax breakdown replacing the old single `tax_kes` field.
    pub tax: TaxBreakdown,
    /// subtotal_kes + tax.vat_kes  (tenant pays this).
    pub total_kes: Decimal,
    pub due_date: Date,
    pub status: InvoiceStatus,
    pub notes: Option<String>,
    /// Set after the invoice is submitted to KRA's eTIMS.
    pub etims_ref: Option<EtimsInvoiceRef>,
    pub created_by: Uuid,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub voided_at: Option<OffsetDateTime>,
}

impl Invoice {
    /// Convenience — gross rent portion of this invoice (lines tagged "rent").
    pub fn gross_rent_kes(&self) -> Decimal {
        self.line_items
            .iter()
            .filter(|l| l.line_type.as_deref() == Some("rent"))
            .map(|l| l.total_kes)
            .sum()
    }

    /// Convenience — management fee portion (lines tagged "management_fee").
    pub fn management_fee_kes(&self) -> Decimal {
        self.line_items
            .iter()
            .filter(|l| l.line_type.as_deref() == Some("management_fee"))
            .map(|l| l.total_kes)
            .sum()
    }

    /// True if a KRA eTIMS reference has been obtained.
    pub fn is_fiscalised(&self) -> bool {
        self.etims_ref.is_some()
    }
}
