use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::vendor::{Vendor, VendorStatus};

#[derive(Debug, Serialize, ToSchema)]
pub struct VendorResponse {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub name: String,
    pub contact_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub speciality: Option<String>,
    pub status: VendorStatus,
    pub notes: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<Vendor> for VendorResponse {
    fn from(v: Vendor) -> Self {
        Self {
            id: v.id,
            agency_id: v.agency_id,
            name: v.name,
            contact_name: v.contact_name,
            email: v.email,
            phone: v.phone,
            speciality: v.speciality,
            status: v.status,
            notes: v.notes,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}
