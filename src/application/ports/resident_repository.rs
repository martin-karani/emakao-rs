use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::{payment::PaymentClaim, resident::{Resident, TenantWithLease}},
};

pub struct CreateResidentCommand {
    pub agency_id: Uuid,
    pub user_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub national_id: Option<String>,
}

#[async_trait]
pub trait ResidentRepository: Send + Sync + 'static {
    async fn find_all(
        &self,
        agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Resident>, AppError>;

    async fn find_by_id(&self, agency_id: Uuid, id: Uuid) -> Result<Option<Resident>, AppError>;
    async fn find_by_email(
        &self,
        agency_id: Uuid,
        email: &str,
    ) -> Result<Option<Resident>, AppError>;
    async fn create(&self, cmd: CreateResidentCommand) -> Result<Resident, AppError>;
    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Option<Resident>, AppError>;
    async fn find_payment_claims_by_resident_id(
        &self,
        resident_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PaymentClaim>, AppError>;

    async fn find_by_property_id(
        &self,
        agency_id: Uuid,
        property_id: Uuid,
    ) -> Result<Vec<Resident>, AppError>;

    async fn find_tenants_by_property(
        &self,
        agency_id: Uuid,
        property_id: Uuid,
    ) -> Result<Vec<TenantWithLease>, AppError>;
}
