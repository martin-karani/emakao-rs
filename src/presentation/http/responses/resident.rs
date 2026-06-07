// ── REFACTORED ────────────────────────────────────────────────────────────────
// Changes from original:
//   - Added rfc3339 serialization to OffsetDateTime fields
//   - Applied #[serde(skip_serializing_if)] to optional fields
// ─────────────────────────────────────────────────────────────────────────────

use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::enums::PortalStatus;
use crate::domain::resident::Resident;

#[derive(Debug, Serialize, ToSchema)]
pub struct ResidentResponse {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Uuid>,
    pub first_name: String,
    pub last_name: String,
    pub full_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub national_id: Option<String>,
    pub portal_status: PortalStatus,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
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
