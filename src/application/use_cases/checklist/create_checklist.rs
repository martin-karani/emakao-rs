
use std::sync::Arc;

use crate::{
    application::{
        errors::AppError,
        ports::checklist_repository::ChecklistRepository,
    },
    domain::checklist::Checklist,
};

pub struct CreateChecklistUseCase {
    pub repo: Arc<dyn ChecklistRepository>,
}

impl CreateChecklistUseCase {
    pub fn new(repo: Arc<dyn ChecklistRepository>) -> Self {
        Self { repo }
    }
}

pub struct CreateChecklistInput {
    pub name: String,
    pub description: Option<String>,
    pub is_default: bool,
}

impl CreateChecklistUseCase {
    pub async fn execute(&self, input: CreateChecklistInput) -> Result<Checklist, AppError> {
        let name = input.name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::Validation("Checklist name cannot be empty".into()));
        }

        self.repo.create_checklist(crate::application::ports::checklist_repository::CreateChecklistCommand {
            name,
            description: input.description,
            is_default: input.is_default,
        }).await
    }
}
