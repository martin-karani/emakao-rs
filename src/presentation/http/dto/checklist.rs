
use garde::Validate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::checklist::ChecklistType;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateChecklistDto {
    #[garde(length(min = 1, max = 200))]
    pub name: String,

    #[garde(skip)]
    pub description: Option<String>,

    #[garde(skip)]
    pub is_default: bool,

    #[garde(skip)]
    pub property_id: Option<Uuid>,

    #[garde(skip)]
    pub template_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateChecklistDto {
    #[garde(length(min = 1, max = 200))]
    pub name: Option<String>,

    #[garde(skip)]
    pub description: Option<String>,

    #[garde(skip)]
    pub is_default: Option<bool>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateChecklistSectionDto {
    #[garde(length(min = 1, max = 200))]
    pub name: String,

    #[garde(skip)]
    pub description: Option<String>,

    #[garde(skip)]
    pub sort_order: i32,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateChecklistItemDto {
    #[garde(length(min = 1, max = 200))]
    pub name: String,

    #[garde(skip)]
    pub description: Option<String>,

    #[garde(skip)]
    pub checklist_type: ChecklistType,

    #[garde(skip)]
    pub sort_order: i32,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AttachChecklistToPropertyDto {
    #[garde(skip)]
    pub checklist_id: Uuid,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct InstantiateChecklistTreeDto {
    #[garde(length(min = 1, max = 200))]
    pub name: String,

    #[garde(skip)]
    pub description: Option<String>,

    #[garde(skip)]
    pub template_id: Option<Uuid>,

    #[garde(skip)]
    pub sections: Vec<InstantiateChecklistSectionDto>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct InstantiateChecklistSectionDto {
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub items: Vec<InstantiateChecklistItemDto>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct InstantiateChecklistItemDto {
    pub name: String,
    pub description: Option<String>,
    pub checklist_type: ChecklistType,
    pub sort_order: i32,
}
