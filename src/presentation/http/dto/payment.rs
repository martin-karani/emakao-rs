use garde::Validate;
use rust_decimal::Decimal;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::domain::{
    enums::{PaymentClaimStatus, PaymentMethodType},
    payment::PaymentAllocationItem,
};

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct SubmitClaimDto {
    #[garde(skip)]
    pub property_id: Uuid,
    #[garde(skip)]
    pub agreement_id: Option<Uuid>,
    #[garde(skip)]
    pub resident_id: Option<Uuid>,
    #[garde(skip)]
    pub unit_id: Option<Uuid>,
    #[garde(skip)]
    pub method_type: PaymentMethodType,
    /// Positive amount validated inside the use case
    #[garde(skip)]
    pub amount_kes: Decimal,
    #[garde(length(min = 1, max = 100))]
    pub reference_code: Option<String>,
    #[garde(length(min = 1, max = 2048))]
    pub proof_url: Option<String>,
    #[garde(length(max = 500))]
    pub notes: Option<String>,
    #[garde(length(min = 1, max = 50))]
    pub submitted_via: Option<String>,
    #[garde(length(max = 5000))]
    pub raw_message: Option<String>,
    #[garde(length(max = 100))]
    pub period_label: Option<String>,
    #[garde(length(max = 100))]
    pub payment_for: Option<String>,
    #[garde(skip)]
    pub allocation: Option<Vec<PaymentAllocationItem>>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ReviewClaimDto {
    #[garde(skip)]
    pub approve: bool,
    #[garde(length(max = 1000))]
    pub review_notes: Option<String>,
    #[garde(length(min = 1, max = 500))]
    pub rejection_reason: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListClaimsParams {
    pub property_id: Option<Uuid>,
    pub status: Option<PaymentClaimStatus>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
