// ── REFACTORED ────────────────────────────────────────────────────────────────
// Changes from original:
//   - Added rfc3339 serialization to OffsetDateTime fields
//   - Applied #[serde(skip_serializing_if)] to optional fields
// ─────────────────────────────────────────────────────────────────────────────

use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::{enums::VendorStatus, vendor::Vendor};

#[derive(Debug, Serialize, ToSchema)]
pub struct VendorResponse {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speciality: Option<String>,
    pub status: VendorStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
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
