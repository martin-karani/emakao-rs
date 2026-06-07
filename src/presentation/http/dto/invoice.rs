use garde::Validate;
use rust_decimal::Decimal;
use serde::Deserialize;
use time::Date;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::domain::enums::InvoiceStatus;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateInvoiceDto {
    #[garde(skip)]
    pub property_id: Uuid,
    #[garde(skip)]
    pub owner_id: Option<Uuid>,
    #[garde(skip)]
    pub agreement_id: Option<Uuid>,
    #[garde(skip)]
    pub resident_id: Option<Uuid>,
    #[garde(length(min = 1))]
    pub line_items: Vec<InvoiceLineItemDto>,
    #[garde(skip)]
    pub due_date: Date,
    #[garde(length(max = 2000))]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct InvoiceLineItemDto {
    #[garde(length(min = 1, max = 200))]
    pub description: String,
    #[garde(skip)]
    pub quantity: Decimal,
    #[garde(skip)]
    pub unit_price_kes: Decimal,
    #[garde(skip)]
    pub line_type: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateInvoiceStatusDto {
    #[garde(skip)]
    pub status: InvoiceStatus,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListInvoicesParams {
    pub property_id: Option<Uuid>,
    pub agreement_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub status: Option<InvoiceStatus>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}
