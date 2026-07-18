
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::checklist_repository::ChecklistRepository,
    },
};

pub struct DeleteChecklistUseCase {
    pub repo: Arc<dyn ChecklistRepository>,
}

impl DeleteChecklistUseCase {
    pub fn new(repo: Arc<dyn ChecklistRepository>) -> Self {
        Self { repo }
    }
}

pub struct DeleteChecklistInput {
    pub id: Uuid,
}

impl DeleteChecklistUseCase {
    pub async fn execute(&self, input: DeleteChecklistInput) -> Result<(), AppError> {
        self.repo.delete_checklist(input.id).await
    }
}
