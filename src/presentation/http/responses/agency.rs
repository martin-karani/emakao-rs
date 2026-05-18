use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::enums::AgencyStatus;

#[derive(Serialize, ToSchema)]
pub struct AgencyResponse {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub schema_name: String,
    pub fga_store_id: Option<String>,
    pub status: AgencyStatus,
}
