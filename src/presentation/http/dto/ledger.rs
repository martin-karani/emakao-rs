use garde::Validate;
use rust_decimal::Decimal;
use serde::Deserialize;
use time::Date;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::domain::ledger::LedgerEntryType;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct PostChargeDto {
    #[garde(skip)]
    pub agreement_id: Option<Uuid>,
    #[garde(skip)]
    pub unit_id: Option<Uuid>,
    #[garde(skip)]
    pub resident_id: Option<Uuid>,
    #[garde(skip)]
    pub entry_type: LedgerEntryType,
    /// Positive amount validated inside the use case
    #[garde(skip)]
    pub amount_kes: Decimal,
    #[garde(length(min = 1, max = 500))]
    pub description: String,
    #[garde(length(max = 100))]
    pub external_ref: Option<String>,
    #[garde(length(max = 50))]
    pub mpesa_receipt: Option<String>,
    #[garde(skip)]
    pub period_start: Option<Date>,
    #[garde(skip)]
    pub period_end: Option<Date>,
    /// Arbitrary JSON metadata blob
    #[garde(skip)]
    #[schema(value_type = Object, nullable = true)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListLedgerParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
