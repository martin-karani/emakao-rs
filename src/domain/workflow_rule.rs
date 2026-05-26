// src/domain/workflow_rule.rs

use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

/// A per-agency automation rule.
///
/// `conditions` holds a JSONLogic predicate evaluated at trigger time.
/// `actions`    is an array of `{ "type": "...", "params": { ... } }` objects.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WorkflowRule {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub name: String,
    pub event_type: String,
    pub is_active: bool,
    /// Positive = hours *after* event, negative = hours *before*.
    pub offset_hours: i32,
    pub conditions: serde_json::Value,
    pub actions: serde_json::Value,
    pub created_at: OffsetDateTime,
}
