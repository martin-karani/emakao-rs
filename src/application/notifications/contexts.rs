//! Typed context structs for every notification template.
//!
//! Each struct serializes to exactly the keys the template consumes —
//! email templates (minijinja) and SMS templates (`SmsTemplate::render`).
//! A missing or mis-spelled key is now a compile error, not a blank field
//! in a delivered message.
//!
//! ## Channel notes
//! * **Shared** — identical keys consumed by both the email jinja template
//!   and the SMS `render()` fn for the same event.
//! * **Email-only** — only an email template exists for this flow
//!   (e.g. `PasswordReset`).
//! * **SMS-only** — only an SMS variant exists (e.g. `Otp`).
//! * **Split** — email and SMS need different fields for the same event
//!   (invite flows: email gets `invite_url`, SMS gets `portal_url` +
//!   `temp_password`).  Separate `*EmailCtx` / `*SmsCtx` structs are used.

use serde::Serialize;

// ── Shared (email + SMS consume the same keys) ────────────────────────────────

/// `EmailTemplate::Welcome` / `SmsTemplate::Welcome`
///
/// Email keys : `first_name`, `login_url`
/// SMS  keys  : `first_name`, `login_url`
#[derive(Debug, Clone, Serialize)]
pub struct WelcomeCtx {
    pub first_name: String,
    pub login_url: String,
}

/// `EmailTemplate::RentDue` / `SmsTemplate::RentDue`
///
/// Email keys : `resident_name`, `amount_kes`, `unit_ref`, `due_date`, `payment_url`
/// SMS  keys  : `resident_name`, `amount_kes`, `unit_ref`, `due_date`, `payment_url`
#[derive(Debug, Clone, Serialize)]
pub struct RentDueCtx {
    pub resident_name: String,
    /// Human-readable KES amount, e.g. `"25,000"`.
    pub amount_kes: String,
    /// Unit identifier shown to the resident, e.g. `"A3"`.
    pub unit_ref: String,
    /// ISO date string `"YYYY-MM-DD"`.
    pub due_date: String,
    /// Deep-link to the payment page; empty string when not yet available.
    pub payment_url: String,
}

/// `EmailTemplate::PaymentConfirmed` / `SmsTemplate::PaymentConfirmed`
///
/// Email keys : `resident_name`, `amount_kes`, `reference`
/// SMS  keys  : `amount_kes`, `reference`, `resident_name`
#[derive(Debug, Clone, Serialize)]
pub struct PaymentConfirmedCtx {
    pub resident_name: String,
    pub amount_kes: String,
    /// Transaction / receipt reference shown to the resident.
    pub reference: String,
}

/// `EmailTemplate::LeaseExpiring` / `SmsTemplate::LeaseExpiring`
///
/// Email keys : `resident_name`, `unit_ref`, `expiry_date`, `days_remaining`
/// SMS  keys  : `resident_name`, `unit_ref`, `days_remaining`, `expiry_date`
#[derive(Debug, Clone, Serialize)]
pub struct LeaseExpiringCtx {
    pub resident_name: String,
    pub unit_ref: String,
    /// ISO date string `"YYYY-MM-DD"`.
    pub expiry_date: String,
    /// Number of days until expiry, as a string e.g. `"14"`.
    pub days_remaining: String,
}

/// `EmailTemplate::WorkOrderUpdate` / `SmsTemplate::WorkOrderUpdate`
///
/// Both channels share the core fields; the email template additionally
/// renders `resident_name`, `vendor_name`, and `description` when present.
///
/// Email keys : `work_order_ref`, `category`, `status`, `scheduled_at`?,
///              `resident_name`?, `vendor_name`?, `description`?
/// SMS  keys  : `work_order_ref`, `category`, `status`, `scheduled_at`?
///
/// **Note:** the call sites previously passed `code` instead of
/// `work_order_ref`.  This struct uses the correct key that matches both the
/// `work_order_update.html.jinja` template and `SmsTemplate::render`.
#[derive(Debug, Clone, Serialize)]
pub struct WorkOrderUpdateCtx {
    /// Human-readable work-order reference, e.g. `"WO-2025-0042"`.
    pub work_order_ref: String,
    pub category: String,
    pub status: String,
    /// ISO datetime or date string; `None` renders nothing in the template.
    pub scheduled_at: Option<String>,
    /// Shown in email greeting; omitted from SMS render.
    pub resident_name: Option<String>,
    /// Assigned contractor; omitted from SMS render.
    pub vendor_name: Option<String>,
    /// Additional notes; omitted from SMS render.
    pub description: Option<String>,
}

// ── Email-only ────────────────────────────────────────────────────────────────

/// `EmailTemplate::PasswordReset`
///
/// Email keys : `first_name`, `reset_url`
///
/// **Note:** the jinja template uses `first_name` (not `full_name`) and
/// `reset_url` (not `reset_link`).  Both were wrong in the previous call site.
#[derive(Debug, Clone, Serialize)]
pub struct PasswordResetCtx {
    /// Pass `user.full_name` or the first token of it — the template renders
    /// it in `"Hi {{ first_name }}"`.
    pub first_name: String,
    /// Full HTTPS reset URL including the token query parameter.
    pub reset_url: String,
}

/// `EmailTemplate::ResidentInvite`
///
/// Email keys : `first_name`, `invite_url`, `agency_name`?
#[derive(Debug, Clone, Serialize)]
pub struct ResidentInviteEmailCtx {
    pub first_name: String,
    pub invite_url: String,
    pub agency_name: Option<String>,
}

/// `EmailTemplate::OwnerInvite`
///
/// Email keys : `first_name`, `invite_url`, `agency_name`?
#[derive(Debug, Clone, Serialize)]
pub struct OwnerInviteEmailCtx {
    pub first_name: String,
    pub invite_url: String,
    pub agency_name: Option<String>,
}

/// `EmailTemplate::VendorInvite`
///
/// Email keys : `contact_name`?, `vendor_name`, `invite_url`, `agency_name`?
#[derive(Debug, Clone, Serialize)]
pub struct VendorInviteEmailCtx {
    /// Individual contact at the vendor company; falls back to `"there"` in
    /// the template when `None`.
    pub contact_name: Option<String>,
    /// Legal / trading name of the vendor business.
    pub vendor_name: String,
    pub invite_url: String,
    pub agency_name: Option<String>,
}

// ── SMS-only ──────────────────────────────────────────────────────────────────

/// `SmsTemplate::Otp`
///
/// SMS keys : `otp`
#[derive(Debug, Clone, Serialize)]
pub struct OtpCtx {
    pub otp: String,
}

/// `SmsTemplate::ResidentInvite`
///
/// SMS keys : `first_name`, `portal_url`, `temp_password`
#[derive(Debug, Clone, Serialize)]
pub struct ResidentInviteSmsCtx {
    pub first_name: String,
    pub portal_url: String,
    pub temp_password: String,
}

/// `SmsTemplate::OwnerInvite`
///
/// SMS keys : `first_name`, `portal_url`, `temp_password`
#[derive(Debug, Clone, Serialize)]
pub struct OwnerInviteSmsCtx {
    pub first_name: String,
    pub portal_url: String,
    pub temp_password: String,
}

/// `SmsTemplate::VendorInvite`
///
#[derive(Debug, Clone, Serialize)]
pub struct VendorInviteSmsCtx {
    pub portal_url: String,
    pub temp_password: String,
}

/// Sent when a platform-admin or agency-admin invites a new staff member
/// (admin / manager / agent) to the staff dashboard.
#[derive(Debug, Clone, Serialize)]
pub struct StaffInviteEmailCtx {
    pub first_name: String,
    pub last_name: String,
    /// Human-readable role label, e.g. `"admin"`, `"manager"`, `"agent"`.
    pub role: String,
    /// Full HTTPS invite URL including the one-time token.
    pub invite_url: String,
    /// Display name of the person who sent the invite.
    pub inviter_name: String,
    /// Agency display name; falls back to `"Emakao"` in the template when `None`.
    pub agency_name: Option<String>,
}
