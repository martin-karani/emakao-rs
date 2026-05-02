use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VendorStatus {
    Active,
    Inactive,
    Blacklisted,
}

#[derive(Clone, Debug, Serialize)]
pub struct Vendor {
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

pub struct CreateVendorCommand {
    pub agency_id: Uuid,
    pub name: String,
    pub contact_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub speciality: Option<String>,
    pub notes: Option<String>,
}

pub struct UpdateVendorCommand {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub name: Option<String>,
    pub contact_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub speciality: Option<String>,
    pub status: Option<VendorStatus>,
    pub notes: Option<String>,
}
