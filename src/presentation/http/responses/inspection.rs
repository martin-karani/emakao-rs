// ── REFACTORED ────────────────────────────────────────────────────────────────
// Changes from original:
//   - Added InspectionItemResponse to own the nested item domain type
//   - Replaced Vec<InspectionItem> with Vec<InspectionItemResponse>
//   - Added rfc3339 serialization to OffsetDateTime fields
//   - Removed created_by (internal audit detail)
//   - Applied #[serde(skip_serializing_if)] to optional fields
// ─────────────────────────────────────────────────────────────────────────────

use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::{
    enums::{InspectionStatus, InspectionType},
    inspection::{Inspection, InspectionItem},
};

#[derive(Debug, Serialize, ToSchema)]
pub struct InspectionItemResponse {
    pub area: String,
    pub condition: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub photo_urls: Vec<String>,
}

impl From<InspectionItem> for InspectionItemResponse {
    fn from(i: InspectionItem) -> Self {
        Self {
            area: i.area,
            condition: i.condition,
            notes: i.notes,
            photo_urls: i.photo_urls,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InspectionResponse {
    pub id: Uuid,
    pub property_id: Uuid,
    pub unit_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agreement_id: Option<Uuid>,
    pub inspection_type: InspectionType,
    pub status: InspectionStatus,
    #[serde(with = "time::serde::rfc3339")]
    pub scheduled_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option", skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conducted_by: Option<Uuid>,
    pub items: Vec<InspectionItemResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_notes: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

// created_by intentionally excluded — internal audit detail.
// Use GET /api/v1/audit-log?entity=inspection&id={id} for actor history.
impl From<Inspection> for InspectionResponse {
    fn from(i: Inspection) -> Self {
        Self {
            id: i.id,
            property_id: i.property_id,
            unit_id: i.unit_id,
            agreement_id: i.agreement_id,
            inspection_type: i.inspection_type,
            status: i.status,
            scheduled_at: i.scheduled_at,
            completed_at: i.completed_at,
            conducted_by: i.conducted_by,
            items: i.items.into_iter().map(InspectionItemResponse::from).collect(),
            summary_notes: i.summary_notes,
            created_at: i.created_at,
            updated_at: i.updated_at,
        }
    }
}
