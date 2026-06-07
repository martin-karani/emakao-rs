use garde::Validate;
use serde::Deserialize;
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::domain::{
    enums::{InspectionStatus, InspectionType},
    inspection::InspectionItem,
};

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateInspectionDto {
    #[garde(skip)]
    pub property_id: Uuid,
    #[garde(skip)]
    pub unit_id: Uuid,
    #[garde(skip)]
    pub agreement_id: Option<Uuid>,
    #[garde(skip)]
    pub inspection_type: InspectionType,
    #[garde(skip)]
    pub scheduled_at: OffsetDateTime,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateInspectionDto {
    #[garde(skip)]
    pub status: Option<InspectionStatus>,
    #[garde(skip)]
    pub scheduled_at: Option<OffsetDateTime>,
    #[garde(skip)]
    pub completed_at: Option<OffsetDateTime>,
    #[garde(skip)]
    pub conducted_by: Option<Option<Uuid>>,
    #[garde(skip)]
    pub items: Option<Vec<InspectionItem>>,
    #[garde(length(max = 5000))]
    pub summary_notes: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListInspectionsParams {
    pub property_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub agreement_id: Option<Uuid>,
    pub status: Option<InspectionStatus>,
    pub inspection_type: Option<InspectionType>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}
