use garde::Validate;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

/// Body for `POST /api/v1/admin/agencies/{agency_id}/staff`
///
/// Platform-admin route — creates a fully-active staff user with a known
/// password.  Use this to seed the first admin after agency provisioning.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateStaffUserDto {
    #[garde(email)]
    pub email: String,

    /// Min 8 characters.
    #[garde(length(min = 8))]
    pub password: String,

    /// One of: `"admin"`, `"manager"`, `"agent"`.
    #[garde(length(min = 1, max = 50))]
    pub role: String,
}

/// Body for `POST /api/v1/staff/invite`
///
/// Agency-admin route — creates an *inactive* staff user and sends an
/// email invite.  The user activates their account by clicking the link
/// and calling `POST /api/v1/auth/accept-invite`.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct InviteStaffDto {
    #[garde(length(min = 1, max = 100))]
    pub first_name: String,

    #[garde(length(min = 1, max = 100))]
    pub last_name: String,

    #[garde(email)]
    pub email: String,

    /// One of: `"admin"`, `"manager"`, `"agent"`.
    #[garde(length(min = 1, max = 50))]
    pub role: String,
}

/// Query params for `GET /api/v1/staff`.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListStaffParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
