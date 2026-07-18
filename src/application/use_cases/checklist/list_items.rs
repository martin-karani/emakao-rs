
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::checklist_repository::ChecklistRepository,
    },
    domain::checklist::ChecklistItem,
};

pub struct ListChecklistItemsUseCase {
    pub repo: Arc<dyn ChecklistRepository>,
}

impl ListChecklistItemsUseCase {
    pub fn new(repo: Arc<dyn ChecklistRepository>) -> Self {
        Self { repo }
    }
}

pub struct ListChecklistItemsInput {
    pub section_id: Uuid,
}

impl ListChecklistItemsUseCase {
    pub async fn execute(&self, input: ListChecklistItemsInput) -> Result<Vec<ChecklistItem>, AppError> {
        self.repo.list_items(input.section_id).await
    }
}
