use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::maintenance_repository::MaintenanceRepository,
    },
    domain::maintenance::{CreateWorkOrderCommand, WorkOrder, WorkOrderPriority},
};

pub struct CreateWorkOrderUseCase {
    pub repo: Arc<dyn MaintenanceRepository>,
}

pub struct CreateWorkOrderInput {
    pub property_id: Uuid,
    pub unit_id: Option<Uuid>,
    pub reported_by: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub priority: WorkOrderPriority,
    pub vendor_id: Option<Uuid>,
}

impl CreateWorkOrderUseCase {
    pub fn new(repo: Arc<dyn MaintenanceRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: CreateWorkOrderInput) -> Result<WorkOrder, AppError> {
        if input.title.trim().is_empty() {
            return Err(AppError::Validation("work order title must not be empty".into()));
        }

        // Uses: MaintenanceRepository::create
        let order = self.repo.create(CreateWorkOrderCommand {
            property_id: input.property_id,
            unit_id: input.unit_id,
            reported_by: input.reported_by,
            title: input.title,
            description: input.description,
            priority: input.priority,
            vendor_id: input.vendor_id,
        }).await?;

        tracing::info!(work_order_id = %order.id, "work order created");
        Ok(order)
    }
}