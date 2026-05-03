use rust_decimal::Decimal;
use serde::Serialize;
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::agreement::{Agreement, AgreementStatus, BillingFrequency};

#[derive(Debug, Serialize, ToSchema)]
pub struct AgreementResponse {
    pub id: Uuid,
    pub property_id: Uuid,
    pub unit_id: Uuid,
    pub resident_id: Uuid,
    pub start_date: Date,
    pub end_date: Option<Date>,
    pub rent_amount_kes: Decimal,
    pub deposit_kes: Decimal,
    pub billing_frequency: BillingFrequency,
    pub status: AgreementStatus,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<Agreement> for AgreementResponse {
    fn from(a: Agreement) -> Self {
        Self {
            id: a.id,
            property_id: a.property_id,
            unit_id: a.unit_id,
            resident_id: a.resident_id,
            start_date: a.start_date,
            end_date: a.end_date,
            rent_amount_kes: a.rent_amount_kes,
            deposit_kes: a.deposit_kes,
            billing_frequency: a.billing_frequency,
            status: a.status,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}
