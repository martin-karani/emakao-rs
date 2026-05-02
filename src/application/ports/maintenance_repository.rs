use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::maintenance::{CreateWorkOrderCommand, UpdateWorkOrderCommand, WorkOrder},
};

#[async_trait]
pub trait MaintenanceRepository: Send + Sync + 'static {
    async fn find_all(
        &self,
        agency_id: Uuid,
        property_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrder>, AppError>;

    async fn find_by_id(
        &self,
        agency_id: Uuid,
        id: Uuid,
    ) -> Result<Option<WorkOrder>, AppError>;

    async fn create(
        &self,
        cmd: CreateWorkOrderCommand,
    ) -> Result<WorkOrder, AppError>;

    async fn update(
        &self,
        cmd: UpdateWorkOrderCommand,
    ) -> Result<WorkOrder, AppError>;
}