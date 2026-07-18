
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::checklist_repository::ChecklistRepository,
    },
};

pub struct DetachChecklistFromPropertyUseCase {
    pub repo: Arc<dyn ChecklistRepository>,
}

impl DetachChecklistFromPropertyUseCase {
    pub fn new(repo: Arc<dyn ChecklistRepository>) -> Self {
        Self { repo }
    }
}

pub struct DetachChecklistFromPropertyInput {
    pub property_id: Uuid,
    pub checklist_id: Uuid,
}

impl DetachChecklistFromPropertyUseCase {
    pub async fn execute(&self, input: DetachChecklistFromPropertyInput) -> Result<(), AppError> {
        self.repo.detach_checklist_from_property(input.property_id, input.checklist_id).await
    }
}
