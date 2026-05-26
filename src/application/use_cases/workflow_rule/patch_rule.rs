// src/application/use_cases/workflow_rule/patch_rule.rs

use std::sync::Arc;

use uuid::Uuid;

use crate::application::{
    errors::AppError,
    ports::workflow_rule_repository::{PatchWorkflowRuleCommand, WorkflowRuleRepository},
};

pub struct PatchRuleInput {
    pub agency_id: Uuid,
    pub rule_id: Uuid,
    pub name: Option<String>,
    pub is_active: Option<bool>,
    pub offset_hours: Option<i32>,
    pub conditions: Option<serde_json::Value>,
    pub actions: Option<serde_json::Value>,
}

pub struct PatchRuleUseCase {
    repo: Arc<dyn WorkflowRuleRepository>,
}

impl PatchRuleUseCase {
    pub fn new(repo: Arc<dyn WorkflowRuleRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: PatchRuleInput) -> Result<(), AppError> {
        // Validate any supplied JSON fields before hitting the database.
        if let Some(ref actions) = input.actions {
            if !actions.is_array() {
                return Err(AppError::Validation(
                    "workflow rule 'actions' must be a JSON array".into(),
                ));
            }
        }
        if let Some(ref conditions) = input.conditions {
            if !conditions.is_object() {
                return Err(AppError::Validation(
                    "workflow rule 'conditions' must be a JSON object".into(),
                ));
            }
        }

        self.repo
            .patch(PatchWorkflowRuleCommand {
                agency_id: input.agency_id,
                rule_id: input.rule_id,
                name: input.name,
                is_active: input.is_active,
                offset_hours: input.offset_hours,
                conditions: input.conditions,
                actions: input.actions,
            })
            .await
    }
}
