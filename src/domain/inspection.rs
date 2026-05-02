use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectionType {
    MoveIn,
    MoveOut,
    Routine,
    Emergency,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectionStatus {
    Scheduled,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Serialize)]
pub struct InspectionItem {
    pub area: String,
    pub condition: String,
    pub notes: Option<String>,
    pub photo_urls: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Inspection {
    pub id: Uuid,
    pub property_id: Uuid,
    pub unit_id: Uuid,
    pub agreement_id: Option<Uuid>,
    pub inspection_type: InspectionType,
    pub status: InspectionStatus,
    pub scheduled_at: OffsetDateTime,
    pub completed_at: Option<OffsetDateTime>,
    pub conducted_by: Option<Uuid>,
    pub items: Vec<InspectionItem>,
    pub summary_notes: Option<String>,
    pub created_by: Uuid,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
