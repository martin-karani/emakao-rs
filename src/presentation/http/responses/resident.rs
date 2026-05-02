use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::resident::{PortalStatus, Resident};

#[derive(Debug, Serialize)]
pub struct ResidentResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub full_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub national_id: Option<String>,
    pub portal_status: PortalStatus,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<Resident> for ResidentResponse {
    fn from(r: Resident) -> Self {
        let full_name = r.full_name();
        Self {
            id: r.id,
            user_id: r.user_id,
            full_name,
            first_name: r.first_name,
            last_name: r.last_name,
            email: r.email,
            phone: r.phone,
            national_id: r.national_id,
            portal_status: r.portal_status,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
