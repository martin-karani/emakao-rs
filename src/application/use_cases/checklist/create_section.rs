
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::checklist_repository::ChecklistRepository,
    },
    domain::checklist::ChecklistSection,
};

pub struct CreateChecklistSectionUseCase {
    pub repo: Arc<dyn ChecklistRepository>,
}

impl CreateChecklistSectionUseCase {
    pub fn new(repo: Arc<dyn ChecklistRepository>) -> Self {
        Self { repo }
    }
}

pub struct CreateChecklistSectionInput {
    pub checklist_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
}

impl CreateChecklistSectionUseCase {
    pub async fn execute(&self, input: CreateChecklistSectionInput) -> Result<ChecklistSection, AppError> {
        let name = input.name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::Validation("Section name cannot be empty".into()));
        }

        self.repo.create_section(crate::application::ports::checklist_repository::CreateChecklistSectionCommand {
            checklist_id: input.checklist_id,
            name,
            description: input.description,
            sort_order: input.sort_order,
        }).await
    }
}
