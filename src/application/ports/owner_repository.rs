use async_trait::async_trait;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::disbursement::Disbursement,
    domain::owner::{CreateOwnerCommand, Owner, UpdateOwnerCommand},
    domain::property::Property,
};

#[async_trait]
pub trait OwnerRepository: Send + Sync + 'static {
    async fn find_all(
        &self,
        agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Owner>, AppError>;
    async fn find_by_id(&self, agency_id: Uuid, id: Uuid) -> Result<Option<Owner>, AppError>;
    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Option<Owner>, AppError>;
    async fn find_by_email(&self, agency_id: Uuid, email: &str) -> Result<Option<Owner>, AppError>;
    async fn create(&self, cmd: CreateOwnerCommand) -> Result<Owner, AppError>;
    async fn update(&self, cmd: UpdateOwnerCommand) -> Result<Owner, AppError>;
    async fn assign_to_property(
        &self,
        owner_id: Uuid,
        property_id: Uuid,
        ownership_percent: Decimal,
    ) -> Result<(), AppError>;
    async fn find_properties_by_owner_id(
        &self,
        owner_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<(Property, Decimal)>, AppError>;
    async fn find_disbursements_by_owner_id(
        &self,
        owner_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Disbursement>, AppError>;
}
