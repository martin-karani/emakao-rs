// src/application/ports/workflow_rule_repository.rs

use async_trait::async_trait;
use uuid::Uuid;

use crate::{application::errors::AppError, domain::workflow_rule::WorkflowRule};

// ── Commands ──────────────────────────────────────────────────────────────────

pub struct CreateWorkflowRuleCommand {
    pub agency_id: Uuid,
    pub name: String,
    pub event_type: String,
    pub is_active: bool,
    pub offset_hours: i32,
    pub conditions: serde_json::Value,
    pub actions: serde_json::Value,
}

pub struct PatchWorkflowRuleCommand {
    pub agency_id: Uuid,
    pub rule_id: Uuid,
    /// Fields absent from the request are `None` and left unchanged.
    pub name: Option<String>,
    pub is_active: Option<bool>,
    pub offset_hours: Option<i32>,
    pub conditions: Option<serde_json::Value>,
    pub actions: Option<serde_json::Value>,
}

// ── Port ──────────────────────────────────────────────────────────────────────

#[async_trait]
pub trait WorkflowRuleRepository: Send + Sync + 'static {
    /// List all rules for an agency, ordered by event_type then name.
    async fn list(&self, agency_id: Uuid) -> Result<Vec<WorkflowRule>, AppError>;

    /// Insert a new rule. Returns `AppError::Conflict` if the name is already taken.
    async fn create(&self, cmd: CreateWorkflowRuleCommand) -> Result<WorkflowRule, AppError>;

    /// Partial-update a rule. Only `Some(_)` fields are written.
    async fn patch(&self, cmd: PatchWorkflowRuleCommand) -> Result<(), AppError>;

    /// Hard-delete a rule. Silently succeeds if the rule does not exist.
    async fn delete(&self, agency_id: Uuid, rule_id: Uuid) -> Result<(), AppError>;
}
