use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::agreement::{Agreement, AgreementStatus},
};

pub struct CreateAgreementCommand {
    pub property_id: Uuid,
    pub unit_id: Uuid,
    pub resident_id: Uuid,
    pub start_date: time::Date,
    pub end_date: Option<time::Date>,
    pub rent_amount_kes: rust_decimal::Decimal,
    pub deposit_kes: rust_decimal::Decimal,
    pub billing_frequency: crate::domain::agreement::BillingFrequency,
}

#[async_trait]
pub trait AgreementRepository: Send + Sync + 'static {
    async fn find_all(
        &self,
        agency_id: Uuid,
        property_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Agreement>, AppError>;

    async fn find_by_id(
        &self,
        agency_id: Uuid,
        id: Uuid,
    ) -> Result<Option<Agreement>, AppError>;

    async fn create(
        &self,
        cmd: CreateAgreementCommand,
    ) -> Result<Agreement, AppError>;

    async fn update_status(
        &self,
        id: Uuid,
        status: AgreementStatus,
        updated_by: Uuid,
    ) -> Result<(), AppError>;

    async fn has_active_agreement(
        &self,
        unit_id: Uuid,
    ) -> Result<bool, AppError>;
}