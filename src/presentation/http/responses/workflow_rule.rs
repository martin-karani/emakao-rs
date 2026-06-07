use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::workflow_rule::WorkflowRule;

#[derive(Debug, Serialize, ToSchema)]
pub struct WorkflowRuleResponse {
    pub id: Uuid,
    pub name: String,
    pub event_type: String,
    pub is_active: bool,
    pub offset_hours: i32,
    pub conditions: serde_json::Value,
    pub actions: serde_json::Value,
}

impl From<WorkflowRule> for WorkflowRuleResponse {
    fn from(r: WorkflowRule) -> Self {
        Self {
            id: r.id,
            name: r.name,
            event_type: r.event_type,
            is_active: r.is_active,
            offset_hours: r.offset_hours,
            conditions: r.conditions,
            actions: r.actions,
        }
    }
}
