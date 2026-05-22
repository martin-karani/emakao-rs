// src/application/notifications/contexts.rs
//
// CHANGED: Added TaxDueCtx at the bottom (all existing structs are unchanged).

use serde::Serialize;

// ── Shared (email + SMS consume the same keys) ────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct WelcomeCtx {
    pub first_name: String,
    pub login_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RentDueCtx {
    pub resident_name: String,
    pub amount_kes: String,
    pub unit_ref: String,
    pub due_date: String,
    pub payment_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentConfirmedCtx {
    pub resident_name: String,
    pub amount_kes: String,
    pub reference: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LeaseExpiringCtx {
    pub resident_name: String,
    pub unit_ref: String,
    pub expiry_date: String,
    pub days_remaining: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkOrderUpdateCtx {
    pub work_order_ref: String,
    pub category: String,
    pub status: String,
    pub scheduled_at: Option<String>,
    pub resident_name: Option<String>,
    pub vendor_name: Option<String>,
    pub description: Option<String>,
}

// ── Email-only ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct PasswordResetCtx {
    pub first_name: String,
    pub reset_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResidentInviteEmailCtx {
    pub first_name: String,
    pub invite_url: String,
    pub agency_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OwnerInviteEmailCtx {
    pub first_name: String,
    pub invite_url: String,
    pub agency_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VendorInviteEmailCtx {
    pub contact_name: Option<String>,
    pub vendor_name: String,
    pub invite_url: String,
    pub agency_name: Option<String>,
}

// ── SMS-only ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct OtpCtx {
    pub otp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResidentInviteSmsCtx {
    pub first_name: String,
    pub portal_url: String,
    pub temp_password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OwnerInviteSmsCtx {
    pub first_name: String,
    pub portal_url: String,
    pub temp_password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VendorInviteSmsCtx {
    pub portal_url: String,
    pub temp_password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StaffInviteEmailCtx {
    pub first_name: String,
    pub last_name: String,
    pub role: String,
    pub invite_url: String,
    pub inviter_name: String,
    pub agency_name: Option<String>,
}

// ── NEW: Tax compliance notifications ─────────────────────────────────────────

/// `EmailTemplate::TaxDue` / `SmsTemplate::TaxDue`
///
/// Sent to agency staff (and optionally the owner) when a tax obligation
/// is due in 5 days or 1 day.
///
/// Email keys : `obligation_type`, `tax_period`, `tax_kes`, `due_date`,
///              `days_until_due`, `owner_name`, `property_address`,
///              `itax_url`
/// SMS  keys  : `obligation_type`, `tax_kes`, `due_date`, `days_until_due`
#[derive(Debug, Clone, Serialize)]
pub struct TaxDueCtx {
    /// Human-readable tax type: "MRI (7.5%)", "VAT (16%)", or "WHT (5%)".
    pub obligation_type: String,
    /// Period label, e.g. "April 2025".
    pub tax_period: String,
    /// Amount in KES formatted as a string, e.g. "7,500".
    pub tax_kes: String,
    /// ISO date string "YYYY-MM-DD".
    pub due_date: String,
    /// "5" or "1".
    pub days_until_due: String,
    /// Owner's full name (for email subject line personalisation).
    pub owner_name: String,
    /// Short property address / reference.
    pub property_address: String,
    /// Deep-link to the KRA iTax portal.
    pub itax_url: String,
}
