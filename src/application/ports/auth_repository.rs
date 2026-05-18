use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{application::errors::AppError, domain::auth::StoredUser};

// ── Commands ──────────────────────────────────────────────────────────────────

pub struct CreateUserCommand {
    pub id: Uuid,
    pub email: Option<String>,
    pub phone: Option<String>,
    /// Empty string for invited-but-not-yet-activated users.
    pub password_hash: String,
    pub is_active: bool,
    pub must_change_password: bool,
}

/// Bind an existing user to an agency with a role.
pub struct CreateMembershipCommand {
    pub user_id: Uuid,
    pub agency_id: Uuid,
    pub role: String,
}

/// Register this contact+portal in the fast-login index.
pub struct UpsertPortalIndexCommand {
    pub contact: String,      // email or E.164 phone
    pub contact_type: String, // "email" | "phone"
    pub portal: String,       // "resident" | "owner" | "vendor"
    pub agency_id: Uuid,
    pub user_id: Uuid,
    pub membership_id: Uuid,
}

/// Token created when a user is invited.
pub struct CreateInviteTokenCommand {
    pub token: String, // 32-char hex
    pub user_id: Uuid,
    pub agency_id: Uuid,
    pub role: String,
    pub portal: String,
    pub contact: String,
    pub contact_type: String,
    /// Only set for phone-only invites — plain-text temp password.
    pub temp_password: Option<String>,
    pub expires_at: OffsetDateTime,
}

// ── Value objects ─────────────────────────────────────────────────────────────

/// Returned by portal-index lookups.
pub struct PortalIdentity {
    pub user_id: Uuid,
    pub agency_id: Uuid,
    pub membership_id: Uuid,
}

/// Returned by consume_invite_token.
pub struct ConsumedInvite {
    pub user_id: Uuid,
    pub agency_id: Uuid,
    pub role: String,
    pub portal: String,
    pub contact: String,
    pub contact_type: String,
}

/// Slim agency row — avoids injecting AgencyRepository into auth use cases.
pub struct SlimAgency {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub schema_name: String,
    pub fga_store_id: Option<String>,
}

// ── Port ──────────────────────────────────────────────────────────────────────

#[async_trait]
pub trait AuthRepository: Send + Sync + 'static {
    // ── User + membership reads ───────────────────────────────────────────────

    /// Staff login: agency is known from slug, look up by email OR phone.
    async fn find_staff_by_contact(
        &self,
        agency_id: Uuid,
        contact: &str,      // normalised email or E.164 phone
        contact_type: &str, // "email" | "phone"
    ) -> Result<Option<StoredUser>, AppError>;

    /// Find a user globally by their primary email.
    async fn find_user_by_email(&self, email: &str) -> Result<Option<StoredUser>, AppError>;

    /// Portal login: look up (contact, portal) in portal_user_index.
    async fn find_portal_identity(
        &self,
        contact: &str,
        contact_type: &str,
        portal: &str,
    ) -> Result<Option<PortalIdentity>, AppError>;

    /// Load user by id (used after portal_identity lookup).
    async fn find_user_by_id(&self, user_id: Uuid) -> Result<Option<StoredUser>, AppError>;

    /// Load membership row so the caller can build JwtClaims.
    async fn find_membership(
        &self,
        user_id: Uuid,
        agency_id: Uuid,
    ) -> Result<Option<(String /* role */, bool /* is_active */)>, AppError>;

    // ── Agency lookup (avoids a second repo dep in auth use cases) ────────────

    async fn find_agency_by_slug(&self, slug: &str) -> Result<Option<SlimAgency>, AppError>;

    async fn find_agency_by_id(&self, id: Uuid) -> Result<Option<SlimAgency>, AppError>;

    async fn contact_exists_for_agency(
        &self,
        agency_id: Uuid,
        contact: &str,
        contact_type: &str,
        role: &str,
    ) -> Result<bool, AppError>;

    async fn create_user(&self, cmd: CreateUserCommand) -> Result<StoredUser, AppError>;

    async fn create_membership(
        &self,
        cmd: CreateMembershipCommand,
    ) -> Result<Uuid /* membership_id */, AppError>;

    /// Set password_hash and is_active = true, must_change_password = false.
    async fn activate_user(&self, user_id: Uuid, password_hash: &str) -> Result<(), AppError>;

    async fn update_last_login(&self, user_id: Uuid) -> Result<(), AppError>;

    // ── Portal index ──────────────────────────────────────────────────────────

    async fn upsert_portal_index(&self, cmd: UpsertPortalIndexCommand) -> Result<(), AppError>;

    async fn remove_portal_index(
        &self,
        contact: &str,
        contact_type: &str,
        portal: &str,
    ) -> Result<(), AppError>;

    // ── Invite tokens ─────────────────────────────────────────────────────────

    async fn create_invite_token(&self, cmd: CreateInviteTokenCommand) -> Result<(), AppError>;

    /// Validate, return, and DELETE the token atomically.
    /// Returns AppError::NotFound if missing or expired.
    async fn consume_invite_token(&self, token: &str) -> Result<ConsumedInvite, AppError>;

    // ── Password reset ────────────────────────────────────────────────────────

    /// Save a one-time password reset token hash.
    async fn save_password_reset_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: OffsetDateTime,
    ) -> Result<(), AppError>;

    /// Find a valid (non-expired) reset token row.
    async fn find_valid_reset_token(
        &self,
        token_hash: &str,
    ) -> Result<Option<PasswordResetToken>, AppError>;

    /// Reset password and delete the token in a transaction.
    async fn reset_password(
        &self,
        user_id: Uuid,
        password_hash: &str,
        token_hash: &str,
    ) -> Result<(), AppError>;

    /// Return `true` if *any* staff role (admin / manager / agent) is already
    /// registered for `email` in this agency.  Used as a duplicate guard before
    /// issuing an invite.
    async fn staff_email_in_agency(&self, agency_id: Uuid, email: &str) -> Result<bool, AppError>;

    /// List all staff members (admin / manager / agent) for an agency,
    /// ordered by creation date descending.
    async fn list_staff(
        &self,
        agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<StaffMember>, AppError>;

    /// Find a single staff member by their `user_agency_roles.id` (membership_id)
    /// scoped to the agency to prevent cross-tenant leakage.
    async fn find_staff_member(
        &self,
        agency_id: Uuid,
        membership_id: Uuid,
    ) -> Result<Option<StaffMember>, AppError>;

    /// Deactivate a staff membership.  Sets `user_agency_roles.is_active = false`.
    /// Does NOT delete — keeps the audit trail.
    async fn deactivate_staff_member(
        &self,
        agency_id: Uuid,
        membership_id: Uuid,
    ) -> Result<(), AppError>;
}

pub struct PasswordResetToken {
    pub user_id: Uuid,
    pub expires_at: OffsetDateTime,
}

pub struct StaffMember {
    pub user_id: Uuid,
    pub membership_id: Uuid,
    pub email: String,
    pub role: String,
    /// Mirror of `users.is_active`.
    pub is_active: bool,
    /// Mirror of `users.must_change_password`.
    pub must_change_password: bool,
    pub created_at: time::OffsetDateTime,
}
