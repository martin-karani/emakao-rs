
use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::checklist::{Checklist, ChecklistItem, ChecklistSection, ChecklistType},
};

pub struct CreateChecklistCommand {
    pub name: String,
    pub description: Option<String>,
    pub is_default: bool,
}

pub struct UpdateChecklistCommand {
    pub id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_default: Option<bool>,
}

pub struct CreateChecklistSectionCommand {
    pub checklist_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
}

pub struct CreateChecklistItemCommand {
    pub section_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub checklist_type: ChecklistType,
    pub sort_order: i32,
}

pub struct InstantiateItemCommand {
    pub name: String,
    pub description: Option<String>,
    pub checklist_type: ChecklistType,
    pub sort_order: i32,
}

pub struct InstantiateSectionCommand {
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub items: Vec<InstantiateItemCommand>,
}

pub struct InstantiateChecklistTreeCommand {
    pub property_id: Uuid,
    pub template_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub sections: Vec<InstantiateSectionCommand>,
}

#[async_trait]
pub trait ChecklistRepository: Send + Sync + 'static {
    async fn create_checklist(&self, cmd: CreateChecklistCommand) -> Result<Checklist, AppError>;
    async fn get_checklist(&self, id: Uuid) -> Result<Checklist, AppError>;
    async fn list_checklists(&self) -> Result<Vec<Checklist>, AppError>;
    async fn update_checklist(&self, cmd: UpdateChecklistCommand) -> Result<Checklist, AppError>;
    async fn delete_checklist(&self, id: Uuid) -> Result<(), AppError>;

    async fn create_section(&self, cmd: CreateChecklistSectionCommand) -> Result<ChecklistSection, AppError>;
    async fn list_sections(&self, checklist_id: Uuid) -> Result<Vec<ChecklistSection>, AppError>;

    async fn create_item(&self, cmd: CreateChecklistItemCommand) -> Result<ChecklistItem, AppError>;
    async fn list_items(&self, section_id: Uuid) -> Result<Vec<ChecklistItem>, AppError>;

    async fn attach_checklist_to_property(&self, property_id: Uuid, checklist_id: Uuid) -> Result<(), AppError>;
    async fn list_property_checklists(&self, property_id: Uuid) -> Result<Vec<Checklist>, AppError>;
    async fn detach_checklist_from_property(&self, property_id: Uuid, checklist_id: Uuid) -> Result<(), AppError>;

    async fn instantiate_tree(&self, cmd: InstantiateChecklistTreeCommand) -> Result<Checklist, AppError>;
}
