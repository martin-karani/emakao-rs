use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::vendor::{CreateVendorCommand, UpdateVendorCommand, Vendor},
};

#[async_trait]
pub trait VendorRepository: Send + Sync + 'static {
    async fn find_all(
        &self,
        agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Vendor>, AppError>;

    async fn find_by_id(
        &self,
        agency_id: Uuid,
        id: Uuid,
    ) -> Result<Option<Vendor>, AppError>;

    async fn create(&self, cmd: CreateVendorCommand) -> Result<Vendor, AppError>;

    async fn update(&self, cmd: UpdateVendorCommand) -> Result<Vendor, AppError>;
}