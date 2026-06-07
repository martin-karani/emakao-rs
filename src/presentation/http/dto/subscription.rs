use garde::Validate;
use serde::Deserialize;
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};

#[derive(Deserialize, ToSchema)]
pub struct ChangePlanDto {
    pub plan_slug: String,
    pub mpesa_ref: Option<String>,
    pub custom_price: Option<i32>,
}

#[derive(Deserialize, ToSchema)]
pub struct CancelDto {
    pub reason: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct SetOverrideDto {
    pub feature_key: String,
    pub value: String,
    pub reason: Option<String>,
    pub expires_at: Option<OffsetDateTime>,
}

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct PaginationQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct InitiatePaymentDto {
    #[garde(length(min = 1))]
    pub plan_slug: String,
    #[garde(length(min = 12, max = 13))]
    pub phone_number: String,
}

fn default_limit() -> i64 {
    20
}
