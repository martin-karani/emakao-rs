use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum VendorStatus {
    Active,
    Inactive,
    Blacklisted,
}

/// Mirrors PortalStatus on residents/owners.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum VendorPortalStatus {
    Invited,
    Active,
    Suspended,
}

#[derive(Clone, Debug, Serialize)]
pub struct Vendor {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub user_id: Option<Uuid>,
    pub name: String,
    pub contact_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub speciality: Option<String>,
    pub status: VendorStatus,
    pub portal_status: VendorPortalStatus,
    pub notes: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

pub struct CreateVendorCommand {
    pub agency_id: Uuid,
    pub user_id: Option<Uuid>,
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
