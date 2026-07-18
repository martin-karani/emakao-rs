
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::checklist_repository::ChecklistRepository,
    },
    domain::checklist::Checklist,
};

pub struct ListPropertyChecklistsUseCase {
    pub repo: Arc<dyn ChecklistRepository>,
}

impl ListPropertyChecklistsUseCase {
    pub fn new(repo: Arc<dyn ChecklistRepository>) -> Self {
        Self { repo }
    }
}

pub struct ListPropertyChecklistsInput {
    pub property_id: Uuid,
}

impl ListPropertyChecklistsUseCase {
    pub async fn execute(&self, input: ListPropertyChecklistsInput) -> Result<Vec<Checklist>, AppError> {
        self.repo.list_property_checklists(input.property_id).await
    }
}
