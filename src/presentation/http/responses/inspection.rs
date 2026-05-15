// src/presentation/http/responses/inspection.rs

use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::{
    enums::{InspectionStatus, InspectionType},
    inspection::{Inspection, InspectionItem},
};

#[derive(Debug, Serialize, ToSchema)]
pub struct InspectionResponse {
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
            items: i.items,
            summary_notes: i.summary_notes,
            created_by: i.created_by,
            created_at: i.created_at,
            updated_at: i.updated_at,
        }
    }
}
