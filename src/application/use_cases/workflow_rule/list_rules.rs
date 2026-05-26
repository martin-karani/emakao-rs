use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::workflow_rule_repository::WorkflowRuleRepository},
    domain::workflow_rule::WorkflowRule,
};

pub struct ListRulesUseCase {
    repo: Arc<dyn WorkflowRuleRepository>,
}

impl ListRulesUseCase {
    pub fn new(repo: Arc<dyn WorkflowRuleRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, agency_id: Uuid) -> Result<Vec<WorkflowRule>, AppError> {
        self.repo.list(agency_id).await
    }
}
