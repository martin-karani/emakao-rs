use garde::Validate;
use rust_decimal::Decimal;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::domain::utility::{BillingMode, MeterType};

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateMeterDto {
    #[garde(skip)]
    pub unit_id: Uuid,
    #[garde(skip)]
    pub meter_type: MeterType,
    #[garde(skip)]
    pub billing_mode: BillingMode,
    #[garde(length(min = 1, max = 50))]
    pub meter_number: String,
    /// Positive rate validated inside the use case
    #[garde(skip)]
    pub rate_per_unit: Decimal,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RecordReadingDto {
    /// Non-negative value validated inside the use case
    #[garde(skip)]
    pub reading_value: Decimal,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListMetersParams {
    pub unit_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListBillsParams {
    pub meter_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
