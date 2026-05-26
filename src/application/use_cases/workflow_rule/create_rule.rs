// src/application/use_cases/workflow_rule/create_rule.rs

use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::workflow_rule_repository::{CreateWorkflowRuleCommand, WorkflowRuleRepository},
    },
    domain::workflow_rule::WorkflowRule,
};

pub struct CreateRuleInput {
    pub agency_id: Uuid,
    pub name: String,
    pub event_type: String,
    pub is_active: bool,
    pub offset_hours: i32,
    pub conditions: serde_json::Value,
    pub actions: serde_json::Value,
}

pub struct CreateRuleUseCase {
    repo: Arc<dyn WorkflowRuleRepository>,
}

impl CreateRuleUseCase {
    pub fn new(repo: Arc<dyn WorkflowRuleRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: CreateRuleInput) -> Result<WorkflowRule, AppError> {
        // Validate that `actions` is a JSON array — enforced here so the
        // repository layer never receives a malformed payload.
        if !input.actions.is_array() {
            return Err(AppError::Validation(
                "workflow rule 'actions' must be a JSON array".into(),
            ));
        }
        if !input.conditions.is_object() {
            return Err(AppError::Validation(
                "workflow rule 'conditions' must be a JSON object".into(),
            ));
        }

        self.repo
            .create(CreateWorkflowRuleCommand {
                agency_id: input.agency_id,
                name: input.name,
                event_type: input.event_type,
                is_active: input.is_active,
                offset_hours: input.offset_hours,
                conditions: input.conditions,
                actions: input.actions,
            })
            .await
    }
}
