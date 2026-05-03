use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::maintenance::{WorkOrder, WorkOrderPriority, WorkOrderStatus};

#[derive(Debug, Serialize, ToSchema)]
pub struct WorkOrderResponse {
    pub id: Uuid,
    pub property_id: Uuid,
    pub unit_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub status: WorkOrderStatus,
    pub priority: WorkOrderPriority,
    pub vendor_id: Option<Uuid>,
    pub reported_by: Uuid,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<WorkOrder> for WorkOrderResponse {
    fn from(w: WorkOrder) -> Self {
        Self {
            id: w.id,
            property_id: w.property_id,
            unit_id: w.unit_id,
            title: w.title,
            description: w.description,
            status: w.status,
            priority: w.priority,
            vendor_id: w.vendor_id,
            reported_by: w.reported_by,
            created_at: w.created_at,
            updated_at: w.updated_at,
        }
    }
}
