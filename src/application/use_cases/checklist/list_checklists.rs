
use std::sync::Arc;

use crate::{
    application::{
        errors::AppError,
        ports::checklist_repository::ChecklistRepository,
    },
    domain::checklist::Checklist,
};

pub struct ListChecklistsUseCase {
    pub repo: Arc<dyn ChecklistRepository>,
}

impl ListChecklistsUseCase {
    pub fn new(repo: Arc<dyn ChecklistRepository>) -> Self {
        Self { repo }
    }
}

pub struct ListChecklistsInput;

impl ListChecklistsUseCase {
    pub async fn execute(&self, _input: ListChecklistsInput) -> Result<Vec<Checklist>, AppError> {
        self.repo.list_checklists().await
    }
}
