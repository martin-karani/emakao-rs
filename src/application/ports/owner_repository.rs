use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::owner::{CreateOwnerCommand, Owner, UpdateOwnerCommand},
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

    async fn find_by_email(&self, agency_id: Uuid, email: &str) -> Result<Option<Owner>, AppError>;

    async fn create(&self, cmd: CreateOwnerCommand) -> Result<Owner, AppError>;

    async fn update(&self, cmd: UpdateOwnerCommand) -> Result<Owner, AppError>;

    async fn assign_to_property(
        &self,
        owner_id: Uuid,
        property_id: Uuid,
        ownership_percent: rust_decimal::Decimal,
    ) -> Result<(), AppError>;
}
