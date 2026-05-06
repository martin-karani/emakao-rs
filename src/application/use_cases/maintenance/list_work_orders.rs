use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::maintenance_repository::MaintenanceRepository},
    domain::maintenance::WorkOrder,
};

pub struct ListWorkOrdersUseCase {
    pub repo: Arc<dyn MaintenanceRepository>,
}

impl ListWorkOrdersUseCase {
    pub fn new(repo: Arc<dyn MaintenanceRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        agency_id: Uuid,
        property_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrder>, AppError> {
        self.repo
            .find_all(agency_id, property_id, limit, offset)
            .await
    }
}
