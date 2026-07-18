
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::checklist_repository::ChecklistRepository,
    },
    domain::checklist::Checklist,
};

pub struct GetChecklistUseCase {
    pub repo: Arc<dyn ChecklistRepository>,
}

impl GetChecklistUseCase {
    pub fn new(repo: Arc<dyn ChecklistRepository>) -> Self {
        Self { repo }
    }
}

pub struct GetChecklistInput {
    pub id: Uuid,
}

impl GetChecklistUseCase {
    pub async fn execute(&self, input: GetChecklistInput) -> Result<Checklist, AppError> {
        self.repo.get_checklist(input.id).await
    }
}
