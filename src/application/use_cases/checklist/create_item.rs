
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::checklist_repository::ChecklistRepository,
    },
    domain::checklist::{ChecklistItem, ChecklistType},
};

pub struct CreateChecklistItemUseCase {
    pub repo: Arc<dyn ChecklistRepository>,
}

impl CreateChecklistItemUseCase {
    pub fn new(repo: Arc<dyn ChecklistRepository>) -> Self {
        Self { repo }
    }
}

pub struct CreateChecklistItemInput {
    pub section_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub checklist_type: ChecklistType,
    pub sort_order: i32,
}

impl CreateChecklistItemUseCase {
    pub async fn execute(&self, input: CreateChecklistItemInput) -> Result<ChecklistItem, AppError> {
        let name = input.name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::Validation("Item name cannot be empty".into()));
        }

        self.repo.create_item(crate::application::ports::checklist_repository::CreateChecklistItemCommand {
            section_id: input.section_id,
            name,
            description: input.description,
            checklist_type: input.checklist_type,
            sort_order: input.sort_order,
        }).await
    }
}
