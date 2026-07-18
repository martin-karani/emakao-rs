
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::checklist_repository::ChecklistRepository,
    },
};

pub struct AttachChecklistToPropertyUseCase {
    pub repo: Arc<dyn ChecklistRepository>,
}

impl AttachChecklistToPropertyUseCase {
    pub fn new(repo: Arc<dyn ChecklistRepository>) -> Self {
        Self { repo }
    }
}

pub struct AttachChecklistToPropertyInput {
    pub property_id: Uuid,
    pub checklist_id: Uuid,
}

impl AttachChecklistToPropertyUseCase {
    pub async fn execute(&self, input: AttachChecklistToPropertyInput) -> Result<(), AppError> {
        self.repo.attach_checklist_to_property(input.property_id, input.checklist_id).await
    }
}
