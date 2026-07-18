
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ChecklistType {
    MoveIn,
    MoveOut,
    Both,
}

impl std::fmt::Display for ChecklistType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChecklistType::MoveIn => write!(f, "move-in"),
            ChecklistType::MoveOut => write!(f, "move-out"),
            ChecklistType::Both => write!(f, "both"),
        }
    }
}

impl std::str::FromStr for ChecklistType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "move-in" => Ok(ChecklistType::MoveIn),
            "move-out" => Ok(ChecklistType::MoveOut),
            "both" => Ok(ChecklistType::Both),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Checklist {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_default: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ChecklistSection {
    pub id: Uuid,
    pub checklist_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ChecklistItem {
    pub id: Uuid,
    pub section_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub checklist_type: ChecklistType,
    pub sort_order: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
