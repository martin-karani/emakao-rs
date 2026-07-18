
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::checklist_repository::ChecklistRepository,
    },
    domain::checklist::Checklist,
};

pub struct UpdateChecklistUseCase {
    pub repo: Arc<dyn ChecklistRepository>,
}

impl UpdateChecklistUseCase {
    pub fn new(repo: Arc<dyn ChecklistRepository>) -> Self {
        Self { repo }
    }
}

pub struct UpdateChecklistInput {
    pub id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_default: Option<bool>,
}

impl UpdateChecklistUseCase {
    pub async fn execute(&self, input: UpdateChecklistInput) -> Result<Checklist, AppError> {
        self.repo.update_checklist(crate::application::ports::checklist_repository::UpdateChecklistCommand {
            id: input.id,
            name: input.name,
            description: input.description,
            is_default: input.is_default,
        }).await
    }
}
