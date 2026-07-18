use garde::Validate;
use serde::Deserialize;
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use rust_decimal::Decimal;

use crate::domain::{
    enums::{InspectionStatus, InspectionType},
    inspection::InspectionItem,
};

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateInspectionDto {
    #[garde(skip)]
    pub property_id: Uuid,
    #[garde(skip)]
    pub unit_id: Uuid,
    #[garde(skip)]
    pub agreement_id: Option<Uuid>,
    #[garde(skip)]
    pub inspection_type: InspectionType,
    #[garde(skip)]
    pub scheduled_at: OffsetDateTime,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateInspectionDto {
    #[garde(skip)]
    pub status: Option<InspectionStatus>,
    #[garde(skip)]
    pub scheduled_at: Option<OffsetDateTime>,
    #[garde(skip)]
    pub completed_at: Option<OffsetDateTime>,
    #[garde(skip)]
    pub conducted_by: Option<Option<Uuid>>,
    #[garde(skip)]
    pub items: Option<Vec<InspectionItem>>,
    #[garde(length(max = 5000))]
    pub summary_notes: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListInspectionsParams {
    pub property_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub agreement_id: Option<Uuid>,
    pub status: Option<InspectionStatus>,
    pub inspection_type: Option<InspectionType>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

// ── Deposit Refund ────────────────────────────────────────────────────────────

/// A single damage / cleaning deduction line item
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RefundDeductionDto {
    #[garde(length(min = 1, max = 200))]
    pub description: String,
    /// Must be > 0; validated in the use case
    #[garde(skip)]
    pub amount_kes: Decimal,
}

/// Request body for POST /api/v1/inspections/{id}/deposit-refund
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ProcessDepositRefundDto {
    /// Agreement the deposit was held against
    #[garde(skip)]
    pub agreement_id: Uuid,
    /// Zero or more damage deductions to subtract from the deposit
    #[garde(skip)]
    pub deductions: Vec<RefundDeductionDto>,
    /// Optional free-text notes (e.g. "tenant agreed to deductions")
    #[garde(length(max = 1000))]
    pub notes: Option<String>,
}
