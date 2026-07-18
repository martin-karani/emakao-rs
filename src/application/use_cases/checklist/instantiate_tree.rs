use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::checklist_repository::{
            ChecklistRepository, InstantiateChecklistTreeCommand, InstantiateItemCommand,
            InstantiateSectionCommand,
        },
    },
    domain::checklist::{Checklist, ChecklistType},
};

pub struct InstantiateChecklistTreeUseCase {
    pub repo: Arc<dyn ChecklistRepository>,
}

impl InstantiateChecklistTreeUseCase {
    pub fn new(repo: Arc<dyn ChecklistRepository>) -> Self {
        Self { repo }
    }
}

pub struct InstantiateChecklistTreeInput {
    pub property_id: Uuid,
    pub template_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub sections: Vec<InstantiateSectionInput>,
}

pub struct InstantiateSectionInput {
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub items: Vec<InstantiateItemInput>,
}

pub struct InstantiateItemInput {
    pub name: String,
    pub description: Option<String>,
    pub checklist_type: ChecklistType,
    pub sort_order: i32,
}

impl InstantiateChecklistTreeUseCase {
    pub async fn execute(&self, input: InstantiateChecklistTreeInput) -> Result<Checklist, AppError> {
        let name = input.name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::Validation(
                "Checklist name cannot be empty".into(),
            ));
        }

        let cmd = InstantiateChecklistTreeCommand {
            property_id: input.property_id,
            template_id: input.template_id,
            name,
            description: input.description,
            sections: input
                .sections
                .into_iter()
                .map(|s| InstantiateSectionCommand {
                    name: s.name,
                    description: s.description,
                    sort_order: s.sort_order,
                    items: s
                        .items
                        .into_iter()
                        .map(|i| InstantiateItemCommand {
                            name: i.name,
                            description: i.description,
                            checklist_type: i.checklist_type,
                            sort_order: i.sort_order,
                        })
                        .collect(),
                })
                .collect(),
        };

        self.repo.instantiate_tree(cmd).await
    }
}
