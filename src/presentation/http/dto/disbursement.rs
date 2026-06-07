use garde::Validate;
use rust_decimal::Decimal;
use serde::Deserialize;
use time::Date;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::domain::enums::{DisbursementMethod, DisbursementStatus};

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateDisbursementDto {
    #[garde(skip)]
    pub owner_id: Uuid,
    #[garde(skip)]
    pub property_id: Uuid,
    #[garde(skip)]
    pub amount_kes: Decimal,
    #[garde(skip)]
    pub method: DisbursementMethod,
    #[garde(skip)]
    pub period_start: Date,
    #[garde(skip)]
    pub period_end: Date,
    #[garde(length(max = 1000))]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateDisbursementStatusDto {
    #[garde(skip)]
    pub status: DisbursementStatus,
    #[garde(length(max = 200))]
    pub reference: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListDisbursementsParams {
    pub owner_id: Option<Uuid>,
    pub property_id: Option<Uuid>,
    pub status: Option<DisbursementStatus>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}
