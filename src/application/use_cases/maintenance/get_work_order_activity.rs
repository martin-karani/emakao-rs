use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::maintenance_repository::MaintenanceRepository},
    domain::maintenance::WorkOrderActivity,
};

pub struct GetWorkOrderActivityUseCase {
    pub repo: Arc<dyn MaintenanceRepository>,
}

impl GetWorkOrderActivityUseCase {
    pub fn new(repo: Arc<dyn MaintenanceRepository>) -> Self {
        Self { repo }
    }

    /// Returns the immutable activity timeline for a single work order,
    /// oldest event first. Only staff routes expose this endpoint; residents
    /// see status changes through the work order status field, not the log.
    pub async fn execute(
        &self,
        agency_id: Uuid,
        work_order_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrderActivity>, AppError> {
        // Scope check: the work order must belong to this agency.
        self.repo
            .find_by_id(agency_id, work_order_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("work order {work_order_id}")))?;

        let limit = limit.clamp(1, 200);
        let offset = offset.max(0);

        self.repo.find_activity(work_order_id, limit, offset).await
    }
}
