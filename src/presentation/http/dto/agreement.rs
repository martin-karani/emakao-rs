use crate::domain::enums::BillingFrequency;
use garde::Validate;
use rust_decimal::Decimal;
use serde::Deserialize;
use time::Date;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateAgreementDto {
    #[garde(skip)]
    pub property_id: Uuid,
    #[garde(skip)]
    pub unit_id: Uuid,
    #[garde(skip)]
    pub resident_id: Uuid,
    #[garde(skip)]
    pub start_date: Date,
    #[garde(skip)]
    pub end_date: Option<Date>,
    #[garde(skip)]
    pub rent_amount_kes: Decimal,
    #[garde(skip)]
    pub deposit_kes: Decimal,
    #[garde(skip)]
    pub billing_frequency: BillingFrequency,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListAgreementsParams {
    pub property_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
