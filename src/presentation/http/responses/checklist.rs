
use crate::domain::checklist::{Checklist, ChecklistItem, ChecklistSection, ChecklistType};
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct ChecklistResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_default: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<Checklist> for ChecklistResponse {
    fn from(checklist: Checklist) -> Self {
        Self {
            id: checklist.id,
            name: checklist.name,
            description: checklist.description,
            is_default: checklist.is_default,
            created_at: checklist.created_at,
            updated_at: checklist.updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChecklistSectionResponse {
    pub id: Uuid,
    pub checklist_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<ChecklistSection> for ChecklistSectionResponse {
    fn from(section: ChecklistSection) -> Self {
        Self {
            id: section.id,
            checklist_id: section.checklist_id,
            name: section.name,
            description: section.description,
            sort_order: section.sort_order,
            created_at: section.created_at,
            updated_at: section.updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChecklistItemResponse {
    pub id: Uuid,
    pub section_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub checklist_type: ChecklistType,
    pub sort_order: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<ChecklistItem> for ChecklistItemResponse {
    fn from(item: ChecklistItem) -> Self {
        Self {
            id: item.id,
            section_id: item.section_id,
            name: item.name,
            description: item.description,
            checklist_type: item.checklist_type,
            sort_order: item.sort_order,
            created_at: item.created_at,
            updated_at: item.updated_at,
        }
    }
}
