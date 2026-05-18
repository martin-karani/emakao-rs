use serde::{Deserialize, Serialize};

use crate::application::errors::AppError;

// ── Email ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmailTemplate {
    Welcome,
    RentDue,
    PaymentConfirmed,
    LeaseExpiring,
    PasswordReset,
    WorkOrderUpdate,
    /// Sent to a newly invited resident (email contact path).
    /// Context: `first_name`, `invite_url`, `agency_name` (optional).
    ResidentInvite,
    /// Sent to a newly onboarded property owner (email contact path).
    /// Context: `first_name`, `invite_url`, `agency_name` (optional).
    OwnerInvite,
    /// Sent to a newly invited vendor/contractor (email contact path).
    /// Context: `contact_name` (optional), `vendor_name`, `invite_url`, `agency_name` (optional).
    VendorInvite,
    /// Sent to a newly invited staff member (admin / manager / agent).
    /// Context: `first_name`, `last_name`, `role`, `invite_url`,
    ///          `inviter_name`, `agency_name` (optional).
    StaffInvite,
}

impl EmailTemplate {
    pub fn template_path(&self) -> &'static str {
        match self {
            Self::Welcome => "email/welcome.html.jinja",
            Self::RentDue => "email/rent_due.html.jinja",
            Self::PaymentConfirmed => "email/payment_confirmed.html.jinja",
            Self::LeaseExpiring => "email/lease_expiring.html.jinja",
            Self::PasswordReset => "email/password_reset.html.jinja",
            Self::WorkOrderUpdate => "email/work_order_update.html.jinja",
            Self::ResidentInvite => "email/resident_invite.html.jinja",
            Self::OwnerInvite => "email/owner_invite.html.jinja",
            Self::VendorInvite => "email/vendor_invite.html.jinja",
            Self::StaffInvite => "email/staff_invite.html.jinja",
        }
    }

    pub fn subject(&self) -> &'static str {
        match self {
            Self::Welcome => "Welcome to Emakao",
            Self::RentDue => "Rent Payment Reminder",
            Self::PaymentConfirmed => "Payment Received – Thank You",
            Self::LeaseExpiring => "Your Lease Is Expiring Soon",
            Self::PasswordReset => "Reset Your Emakao Password",
            Self::WorkOrderUpdate => "Work Order Status Update",
            Self::ResidentInvite => "Welcome to Emakao – Resident Portal Invitation",
            Self::OwnerInvite => "Welcome to Emakao – Activate Your Owner Portal",
            Self::VendorInvite => "You've Been Added to the Emakao Vendor Directory",
            Self::StaffInvite => "You've been invited to the Emakao Staff Dashboard",
        }
    }
}

// ── SMS ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmsTemplate {
    Welcome,
    RentDue,
    PaymentConfirmed,
    LeaseExpiring,
    Otp,
    WorkOrderUpdate,
    /// Sent to a newly invited resident (phone contact path).
    ResidentInvite,
    /// Sent to a newly onboarded property owner (phone contact path).
    OwnerInvite,
    /// Sent to a newly invited vendor/contractor (phone contact path).
    VendorInvite,
    // Note: StaffInvite has no SMS variant — staff always use email.
}

impl SmsTemplate {
    pub fn render(&self, ctx: &serde_json::Value) -> Result<String, AppError> {
        let get = |key: &str| -> &str { ctx.get(key).and_then(|v| v.as_str()).unwrap_or("") };

        let msg = match self {
            Self::Welcome => format!(
                "Emakao: Welcome {}! Your account is ready. Login: {}",
                get("first_name"),
                get("login_url"),
            ),
            Self::RentDue => format!(
                "Emakao: Hi {}, rent KES {} for {} is due {}. Pay: {}",
                get("resident_name"),
                get("amount_kes"),
                get("unit_ref"),
                get("due_date"),
                get("payment_url"),
            ),
            Self::PaymentConfirmed => format!(
                "Emakao: Payment KES {} received. Ref: {}. Thank you, {}!",
                get("amount_kes"),
                get("reference"),
                get("resident_name"),
            ),
            Self::LeaseExpiring => format!(
                "Emakao: Hi {}, your lease for {} expires in {} days ({}). Contact your manager.",
                get("resident_name"),
                get("unit_ref"),
                get("days_remaining"),
                get("expiry_date"),
            ),
            Self::Otp => format!(
                "Emakao: Your code is {}. Valid 10 mins. Do not share.",
                get("otp"),
            ),
            Self::WorkOrderUpdate => {
                let base = format!(
                    "Emakao: Work order #{} ({}) is now {}.",
                    get("work_order_ref"),
                    get("category"),
                    get("status"),
                );
                match ctx
                    .get("scheduled_at")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                {
                    Some(date) => format!("{base} Scheduled: {date}."),
                    None => base,
                }
            }
            Self::ResidentInvite => format!(
                "Hi {}, you've been invited to the Emakao resident portal at {}. Temp password: {}. Change it after first login.",
                get("first_name"),
                get("portal_url"),
                get("temp_password"),
            ),
            Self::OwnerInvite => format!(
                "Hi {}, welcome to the Emakao owner portal at {}. Temp password: {}. Change it on first login.",
                get("first_name"),
                get("portal_url"),
                get("temp_password"),
            ),
            Self::VendorInvite => format!(
                "You've been added to the Emakao vendor portal at {}. Temp password: {}. Change it on first login.",
                get("portal_url"),
                get("temp_password"),
            ),
        };

        Ok(msg)
    }
}
