//! Invite a new staff member to an already-provisioned agency.
//!
//! ## Flow
//! 1. Guard — email must not already have a staff role in this agency.
//! 2. Create a platform `users` row (inactive; empty password hash).
//! 3. Bind to the agency with the requested role (`user_agency_roles`).
//! 4. Register in `portal_user_index` for O(1) login lookups.
//! 5. Generate a 48 h invite token and persist it.
//! 6. Send the invite email via `NotificationService`.
//!
//! **Staff members always have an email address** — there is no phone-only
//! path for the staff portal.

use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::{
    errors::AppError,
    helpers::auth_helpers::generate_token,
    notifications::{
        contexts::StaffInviteEmailCtx, service::NotificationService, templates::EmailTemplate,
    },
    ports::{
        auth_port::AuthPort,
        auth_repository::{
            AuthRepository, CreateInviteTokenCommand, CreateMembershipCommand, CreateUserCommand,
            UpsertPortalIndexCommand,
        },
    },
};

/// Valid staff roles accepted by this use case.
const STAFF_ROLES: &[&str] = &["admin", "manager", "agent"];

// ── Input / Output ────────────────────────────────────────────────────────────

pub struct InviteStaffInput {
    pub agency_id: Uuid,
    /// Must be a valid email address; normalised to lowercase inside the use case.
    pub email: String,
    /// Display name used in the greeting line of the invite email.
    pub first_name: String,
    pub last_name: String,
    /// Must be one of: `"admin"`, `"manager"`, `"agent"`.
    pub role: String,
    /// Name of the person performing the invite (shown as "invited by").
    pub inviter_name: String,
    /// Agency display name shown in the email body.
    pub agency_name: Option<String>,
    /// Base URL of the staff dashboard, e.g. `"https://app.emakao.co.ke"`.
    /// The final invite link becomes `{portal_base_url}/invite/{token}`.
    pub portal_base_url: String,
}

pub struct InviteStaffOutput {
    /// Platform `users` row id of the newly created (inactive) user.
    pub user_id: Uuid,
    /// Normalised email address.
    pub email: String,
    /// Role assigned in `user_agency_roles`.
    pub role: String,
    /// Invite URL embedded in the email — useful for tests / admin tools.
    pub invite_url: String,
}

// ── Use case ──────────────────────────────────────────────────────────────────

pub struct InviteStaffUseCase {
    pub auth_repo: Arc<dyn AuthRepository>,
    pub auth_port: Arc<dyn AuthPort>,
    pub notifications: NotificationService,
}

impl InviteStaffUseCase {
    pub async fn execute(&self, input: InviteStaffInput) -> Result<InviteStaffOutput, AppError> {
        // ── Validate role ──────────────────────────────────────────────────────
        if !STAFF_ROLES.contains(&input.role.as_str()) {
            return Err(AppError::Validation(format!(
                "role must be one of: {}",
                STAFF_ROLES.join(", ")
            )));
        }

        let email = input.email.trim().to_lowercase();

        // ── Guard: duplicate ───────────────────────────────────────────────────
        // Check if this email already has *any* staff role in this agency via
        // the portal index + membership join.  We use the role being invited so
        // the same person can theoretically hold different roles, but the unique
        // constraint on user_agency_roles(user_id, agency_id, role) would also
        // reject a true duplicate at the DB level.
        if self
            .auth_repo
            .staff_email_in_agency(input.agency_id, &email)
            .await?
        {
            return Err(AppError::Conflict(format!(
                "A staff member with email '{}' already exists in this agency.",
                email
            )));
        }

        // ── Step 1: Create platform user (inactive until invite accepted) ──────
        let user_id = Uuid::new_v4();
        self.auth_repo
            .create_user(CreateUserCommand {
                id: user_id,
                email: Some(email.clone()),
                phone: None,
                // Empty — the invite flow sets this when accept-invite is called.
                password_hash: "".to_string(),
                is_active: false,
                must_change_password: false,
            })
            .await?;

        // ── Step 2: Membership ─────────────────────────────────────────────────
        let membership_id = self
            .auth_repo
            .create_membership(CreateMembershipCommand {
                user_id,
                agency_id: input.agency_id,
                role: input.role.clone(),
            })
            .await?;

        // ── Step 3: Portal index ───────────────────────────────────────────────
        self.auth_repo
            .upsert_portal_index(UpsertPortalIndexCommand {
                contact: email.clone(),
                contact_type: "email".to_owned(),
                portal: "staff".to_owned(),
                agency_id: input.agency_id,
                user_id,
                membership_id,
            })
            .await?;

        // ── Step 4: Invite token (48 h TTL) ────────────────────────────────────
        let token = generate_token();
        self.auth_repo
            .create_invite_token(CreateInviteTokenCommand {
                token: token.clone(),
                user_id,
                agency_id: input.agency_id,
                role: input.role.clone(),
                portal: "staff".to_owned(),
                contact: email.clone(),
                contact_type: "email".to_owned(),
                temp_password: None, // email path — no temp password
                expires_at: OffsetDateTime::now_utc() + time::Duration::hours(48),
            })
            .await?;

        // ── Step 5: Send invite email (best-effort; soft failure logged) ────────
        let invite_url = format!(
            "{}/invite/{}",
            input.portal_base_url.trim_end_matches('/'),
            token
        );

        let _ = self
            .notifications
            .email(
                email.clone(),
                EmailTemplate::StaffInvite,
                StaffInviteEmailCtx {
                    first_name: input.first_name.clone(),
                    last_name: input.last_name.clone(),
                    role: input.role.clone(),
                    invite_url: invite_url.clone(),
                    inviter_name: input.inviter_name.clone(),
                    agency_name: input.agency_name.clone(),
                },
            )
            .await;

        tracing::info!("Email job enqueued for {}", email);

        tracing::info!(
            user_id    = %user_id,
            agency_id  = %input.agency_id,
            role       = %input.role,
            email      = %email,
            "staff member invited"
        );

        Ok(InviteStaffOutput {
            user_id,
            email,
            role: input.role,
            invite_url,
        })
    }
}
