
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::checklist_repository::ChecklistRepository,
    },
    domain::checklist::ChecklistSection,
};

pub struct ListChecklistSectionsUseCase {
    pub repo: Arc<dyn ChecklistRepository>,
}

impl ListChecklistSectionsUseCase {
    pub fn new(repo: Arc<dyn ChecklistRepository>) -> Self {
        Self { repo }
    }
}

pub struct ListChecklistSectionsInput {
    pub checklist_id: Uuid,
}

impl ListChecklistSectionsUseCase {
    pub async fn execute(&self, input: ListChecklistSectionsInput) -> Result<Vec<ChecklistSection>, AppError> {
        self.repo.list_sections(input.checklist_id).await
    }
}
