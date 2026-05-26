use std::sync::Arc;

use uuid::Uuid;

use crate::application::{
    errors::AppError, ports::workflow_rule_repository::WorkflowRuleRepository,
};

pub struct DeleteRuleUseCase {
    repo: Arc<dyn WorkflowRuleRepository>,
}

impl DeleteRuleUseCase {
    pub fn new(repo: Arc<dyn WorkflowRuleRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, agency_id: Uuid, rule_id: Uuid) -> Result<(), AppError> {
        self.repo.delete(agency_id, rule_id).await
    }
}
