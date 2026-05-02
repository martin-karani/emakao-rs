use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::maintenance_repository::MaintenanceRepository},
    domain::maintenance::{UpdateWorkOrderCommand, WorkOrder, WorkOrderStatus},
};

pub struct UpdateWorkOrderUseCase {
    pub repo: Arc<dyn MaintenanceRepository>,
}

pub struct UpdateWorkOrderInput {
    pub agency_id: Uuid,
    pub work_order_id: Uuid,
    pub status: Option<WorkOrderStatus>,
    pub vendor_id: Option<Option<Uuid>>,
    pub description: Option<String>,
}

impl UpdateWorkOrderUseCase {
    pub fn new(repo: Arc<dyn MaintenanceRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: UpdateWorkOrderInput) -> Result<WorkOrder, AppError> {
        // Uses: MaintenanceRepository::find_by_id — guard existence
        self.repo
            .find_by_id(input.agency_id, input.work_order_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("work order {}", input.work_order_id)))?;

        // Uses: MaintenanceRepository::update
        self.repo.update(UpdateWorkOrderCommand {
            id: input.work_order_id,
            agency_id: input.agency_id,
            status: input.status,
            vendor_id: input.vendor_id,
            description: input.description,
        }).await
    }
}