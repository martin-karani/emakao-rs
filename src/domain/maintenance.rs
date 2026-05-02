use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkOrderStatus {
    Open,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkOrderPriority {
    Low,
    Medium,
    High,
    Emergency,
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkOrder {
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

/// Command passed to `MaintenanceRepository::create`.
pub struct CreateWorkOrderCommand {
    pub property_id: Uuid,
    pub unit_id: Option<Uuid>,
    pub reported_by: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub priority: WorkOrderPriority,
    pub vendor_id: Option<Uuid>,
}

/// Command passed to `MaintenanceRepository::update`.
pub struct UpdateWorkOrderCommand {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub status: Option<WorkOrderStatus>,
    pub vendor_id: Option<Option<Uuid>>, // Some(Some(id)) = set, Some(None) = clear
    pub description: Option<String>,
}
