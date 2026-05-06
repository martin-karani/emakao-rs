use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::maintenance_repository::MaintenanceRepository},
    domain::maintenance::WorkOrder,
};

pub struct GetWorkOrderUseCase {
    pub repo: Arc<dyn MaintenanceRepository>,
}

impl GetWorkOrderUseCase {
    pub fn new(repo: Arc<dyn MaintenanceRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, agency_id: Uuid, id: Uuid) -> Result<WorkOrder, AppError> {
        // Uses: MaintenanceRepository::find_by_id
        self.repo
            .find_by_id(agency_id, id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("work order {id}")))
    }
}
