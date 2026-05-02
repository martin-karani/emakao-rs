use garde::Validate;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::maintenance::{WorkOrderPriority, WorkOrderStatus};

#[derive(Debug, Deserialize, Validate)]
pub struct CreateWorkOrderDto {
    #[garde(skip)]
    pub property_id: Uuid,
    #[garde(skip)]
    pub unit_id: Option<Uuid>,
    #[garde(length(min = 1, max = 200))]
    pub title: String,
    #[garde(length(max = 2000))]
    pub description: Option<String>,
    #[garde(skip)]
    pub priority: WorkOrderPriority,
    #[garde(skip)]
    pub vendor_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateWorkOrderDto {
    #[garde(skip)]
    pub status: Option<WorkOrderStatus>,
    /// `Some(Some(id))` = assign vendor, `Some(None)` = clear vendor
    #[garde(skip)]
    pub vendor_id: Option<Option<Uuid>>,
    #[garde(length(max = 2000))]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListWorkOrdersParams {
    pub property_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
