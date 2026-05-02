use garde::Validate;
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::payment::{ClaimStatus, PaymentMethodType};

#[derive(Debug, Deserialize, Validate)]
pub struct SubmitClaimDto {
    #[garde(skip)]
    pub property_id: Uuid,
    #[garde(skip)]
    pub agreement_id: Option<Uuid>,
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
}

#[derive(Debug, Deserialize, Validate)]
pub struct ReviewClaimDto {
    #[garde(skip)]
    pub approve: bool,
    #[garde(length(max = 1000))]
    pub review_notes: Option<String>,
    #[garde(length(min = 1, max = 500))]
    pub rejection_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListClaimsParams {
    pub property_id: Option<Uuid>,
    pub status: Option<ClaimStatus>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
