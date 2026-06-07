// ── REFACTORED ────────────────────────────────────────────────────────────────
// Changes from original:
//   - Added rfc3339 serialization to OffsetDateTime fields
//   - Applied #[serde(skip_serializing_if)] to optional fields
// ─────────────────────────────────────────────────────────────────────────────

use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::application::ports::auth_repository::StaffMember;

/// Returned by all staff management endpoints.
#[derive(Debug, Serialize, ToSchema)]
pub struct StaffUserResponse {
    /// `user_agency_roles.id` — use this as the resource ID for
    /// `GET /api/v1/staff/{id}` and `DELETE /api/v1/staff/{id}`.
    pub membership_id: Uuid,
    pub user_id: Uuid,
    pub email: String,
    /// One of: `"admin"`, `"manager"`, `"agent"`.
    pub role: String,
    /// `true` once the user has accepted the invite and set a password.
    pub is_active: bool,
    /// `true` if the frontend should redirect to /change-password on next login.
    pub must_change_password: bool,
    /// When the user row was created (= when the invite was sent for the invite flow).
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl From<StaffMember> for StaffUserResponse {
    fn from(m: StaffMember) -> Self {
        Self {
            membership_id: m.membership_id,
            user_id: m.user_id,
            email: m.email,
            role: m.role,
            is_active: m.is_active,
            must_change_password: m.must_change_password,
            created_at: m.created_at,
        }
    }
}

/// Returned by `POST /api/v1/admin/agencies/{agency_id}/staff` (direct creation).
#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedStaffUserResponse {
    pub user_id: Uuid,
    pub email: String,
    pub role: String,
    /// Always `true` for the platform-admin direct-creation path.
    pub is_active: bool,
    /// Callers should force the user to change password on first login.
    pub must_change_password: bool,
}

/// Returned by `POST /api/v1/staff/invite`.
#[derive(Debug, Serialize, ToSchema)]
pub struct InviteStaffResponse {
    pub user_id: Uuid,
    pub email: String,
    pub role: String,
    /// Always `false` — the account is inactive until the invite is accepted.
    pub is_active: bool,
    /// Primarily for admin tooling / e2e tests; omit from production API docs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invite_url: Option<String>,
}
