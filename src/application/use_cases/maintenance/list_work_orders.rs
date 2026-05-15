use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::maintenance_repository::MaintenanceRepository},
    domain::{
        enums::{WorkOrderCategory, WorkOrderPriority, WorkOrderReporterType, WorkOrderStatus},
        maintenance::WorkOrder,
    },
};

pub struct ListWorkOrdersUseCase {
    pub repo: Arc<dyn MaintenanceRepository>,
}

impl ListWorkOrdersUseCase {
    pub fn new(repo: Arc<dyn MaintenanceRepository>) -> Self {
        Self { repo }
    }
}

pub struct ListWorkOrdersInput {
    pub agency_id: Uuid,

    // ── Scope filters ─────────────────────────────────────────────────────────
    /// Narrow to a single property.
    pub property_id: Option<Uuid>,
    /// Narrow to a single unit within a property.
    pub unit_id: Option<Uuid>,

    // ── Field filters ─────────────────────────────────────────────────────────
    pub status: Option<WorkOrderStatus>,
    pub priority: Option<WorkOrderPriority>,
    pub category: Option<WorkOrderCategory>,
    /// Show only orders submitted by a particular type of actor.
    pub reporter_type: Option<WorkOrderReporterType>,
    /// Show only orders assigned to a specific caretaker.
    pub assigned_caretaker_id: Option<Uuid>,

    // ── Pagination ────────────────────────────────────────────────────────────
    pub limit: i64,
    pub offset: i64,
}

impl ListWorkOrdersUseCase {
    pub async fn execute(&self, input: ListWorkOrdersInput) -> Result<Vec<WorkOrder>, AppError> {
        // Guard pagination bounds — never trust callers to stay sane.
        let limit = input.limit.clamp(1, 100);
        let offset = input.offset.max(0);

        self.repo
            .find_all(
                input.agency_id,
                input.property_id,
                input.unit_id,
                input.status,
                input.priority,
                input.category,
                input.reporter_type,
                input.assigned_caretaker_id,
                limit,
                offset,
            )
            .await
    }
}
