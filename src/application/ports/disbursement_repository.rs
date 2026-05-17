use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::{
        disbursement::{CreateDisbursementCommand, Disbursement},
        enums::DisbursementStatus,
    },
};

pub struct DisbursementFilter {
    pub owner_id: Option<Uuid>,
    pub property_id: Option<Uuid>,
    pub status: Option<DisbursementStatus>,
    pub limit: i64,
    pub offset: i64,
}

#[async_trait]
pub trait DisbursementRepository: Send + Sync + 'static {
    async fn find_all(
        &self,
        agency_id: Uuid,
        filter: DisbursementFilter,
    ) -> Result<Vec<Disbursement>, AppError>;

    async fn find_by_id(&self, agency_id: Uuid, id: Uuid)
        -> Result<Option<Disbursement>, AppError>;

    async fn create(&self, cmd: CreateDisbursementCommand) -> Result<Disbursement, AppError>;

    async fn update_status(
        &self,
        agency_id: Uuid,
        id: Uuid,
        status: DisbursementStatus,
        reference: Option<String>,
    ) -> Result<Disbursement, AppError>;
}
