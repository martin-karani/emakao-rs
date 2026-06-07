use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::enums::AgencyStatus;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Agency {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub schema_name: String,
    pub country_code: String,
    pub currency_code: String,
    pub fga_store_id: Option<String>,
    pub status: AgencyStatus,
}

/// Injected by `resolve_agency_context` into every authenticated request's extensions.
///
/// Portal type is NOT here — it lives in `AuthenticatedUser.portal` so a
/// single `ResolvedAgency` can serve requests from any portal type on the
/// same agency.
#[derive(Clone, Debug)]
pub struct ResolvedAgency {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub schema_name: String,
    pub fga_store_id: Option<String>,
}

// ── Agency Integration ───────────────────────────────────────────────────────────

/// Redacted summary of a provider integration.
///
/// Credentials are stored AES-256-GCM encrypted and are **never** included
/// in this type — the `credentials` column is write-only from the application
/// layer's perspective.
#[derive(Debug, Clone, Serialize)]
pub struct AgencyIntegration {
    pub agency_id: Uuid,
    /// e.g. "payment" | "sms" | "email" | "storage"
    pub provider_type: String,
    /// e.g. "mpesa" | "africas_talking" | "sendgrid"
    pub provider_key: String,
    pub is_active: bool,
    /// Non-secret config: shortcodes, sender IDs, bucket names.
    pub settings: serde_json::Value,
}

// ── Agency Settings ─────────────────────────────────────────────────────────────

/// Top-level aggregator — deserialised once per request (or from cache).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AgencySettings {
    pub branding: BrandingSettings,
    pub communication: CommunicationSettings,
    pub workflows: WorkflowSettings,
    pub ui: UiSettings,
    pub portal: PortalSettings,
    pub onboarding: OnboardingSettings,
    /// Staging area for new settings.  Values here are accessed via
    /// `AgencySettings::extra_bool` / `extra_i64` helpers below.
    /// Never remove this field — it is the safe landing zone for every
    /// new setting before it earns a typed column.
    pub extra: serde_json::Value,
}

// ── Branding ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct BrandingSettings {
    pub logo_light_url: Option<String>,
    pub logo_dark_url: Option<String>,
    pub favicon_url: Option<String>,
    /// CSS hex colour, e.g. "#1A56DB"
    pub primary_color: String,
    pub secondary_color: String,
    pub accent_color: String,
    pub font_family: String,
    pub trading_name: Option<String>,
    /// The "From" name on outbound emails.
    pub email_from_name: String,
    pub email_reply_to: Option<String>,
    /// Alphanumeric SMS sender ID — max 11 characters (carrier limit).
    pub sms_sender_id: Option<String>,
    pub whatsapp_number: Option<String>,
    /// `<agency>.app.emakao.co.ke` — set at provisioning time.
    pub portal_subdomain: Option<String>,
    /// Custom CNAME domain after TLS cert is verified.
    pub portal_custom_domain: Option<String>,
    pub footer_disclaimer: Option<String>,
    /// MiniJinja HTML override for PDF invoices.  Falls back to the system
    /// default when `None`.
    pub invoice_template: Option<String>,
    /// MiniJinja HTML override for PDF statements.
    pub statement_template: Option<String>,
}

// ── Communication ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CommunicationSettings {
    /// Days before rent due date to send reminders, e.g. `[7, 3, 1]`.
    pub rent_reminder_days: Vec<i32>,
    /// Days before lease end date to send renewal notice.
    pub lease_renewal_notice_days: i32,
    /// Local hour (0–23) before which no SMS is sent.
    pub quiet_start_hour: u8,
    /// Local hour (0–23) after which no SMS is sent.
    pub quiet_end_hour: u8,
    /// Maximum reminders per billing cycle event.
    pub max_reminders_per_event: u8,
    /// Additional per-event SMS rules beyond the standard reminder schedule.
    pub sms_trigger_rules: Vec<SmsRule>,
    /// Optional escalation chain triggered after N hours without resolution.
    pub escalation_policy: Option<EscalationPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SmsRule {
    pub event_key: String,
    pub template: String,
    pub offset_days: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct EscalationPolicy {
    pub after_hours: i32,
    pub notify_user_id: Option<Uuid>,
    /// "sms" | "email"
    pub channel: String,
}

// ── Workflows / Business Rules ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct WorkflowSettings {
    pub late_fee_grace_days: i32,
    /// "flat" | "percent"
    pub late_fee_type: String,
    pub late_fee_value: Decimal,
    pub late_fee_max_kes: Option<Decimal>,
    /// "oldest_first" | "penalties_first" | "rent_first"
    pub payment_allocation_strategy: String,
    pub deposit_months: Decimal,
    pub tenant_notice_days: i32,
    pub auto_renew_leases: bool,
    /// "none" | "fixed_percent" | "cpi"
    pub annual_escalation_type: String,
    pub annual_escalation_value: Decimal,
    /// Work orders below this threshold are auto-approved by staff.
    pub auto_approve_wo_below_kes: Decimal,
    pub maintenance_requires_owner_approval: bool,
    /// Day of month rent is due (1–28).
    pub rent_due_day: u8,
    pub billing_currency: String,
}

// ── UI Preferences ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct UiSettings {
    /// e.g. "dashboard" | "properties" | "agreements"
    pub default_landing: String,
    /// e.g. "DD/MM/YYYY"
    pub date_format: String,
    /// "sqm" | "sqft"
    pub area_unit: String,
    /// Ordered widget keys shown on the dashboard.
    pub dashboard_widgets: Vec<String>,
    /// Theme override key, e.g. "dark" | "light" | "system"
    pub theme: Option<String>,
}

// ── Resident Portal ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PortalSettings {
    pub resident_can_submit_maintenance: bool,
    pub resident_can_view_statements: bool,
    pub resident_can_download_invoices: bool,
    pub resident_can_chat: bool,
    pub owner_can_view_financials: bool,
    pub owner_can_approve_work_orders: bool,
}

// ── Onboarding Defaults ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct OnboardingSettings {
    /// Role assigned to new staff invitations when not specified.
    pub default_staff_role: String,
    /// Default property type seeded on agency creation.
    pub default_property_type: Option<String>,
    /// MiniJinja SMS template body for new resident welcome.
    pub welcome_sms_template: Option<String>,
    /// MiniJinja email template key for new staff welcome.
    pub welcome_email_template: Option<String>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

impl AgencySettings {
    /// Deserialise from a JSONB value, falling back to `Default` on any error.
    /// Use this everywhere instead of calling `serde_json::from_value` directly.
    pub fn from_jsonb(json: &serde_json::Value) -> Self {
        serde_json::from_value(json.clone()).unwrap_or_default()
    }

    /// Read a boolean from the `extra` staging area.
    pub fn extra_bool(&self, key: &str, default: bool) -> bool {
        self.extra
            .get(key)
            .and_then(|v| v.as_bool())
            .unwrap_or(default)
    }

    /// Read an integer from the `extra` staging area.
    pub fn extra_i64(&self, key: &str, default: i64) -> i64 {
        self.extra
            .get(key)
            .and_then(|v| v.as_i64())
            .unwrap_or(default)
    }

    /// Read a string from the `extra` staging area.
    pub fn extra_str<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.extra
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or(default)
    }

    /// Returns true if the SMS quiet-hours window is currently active.
    /// `hour` is the local hour (0–23) at the agency's timezone.
    pub fn in_quiet_hours(&self, hour: u8) -> bool {
        let start = self.communication.quiet_start_hour;
        let end = self.communication.quiet_end_hour;
        if end > start {
            // Normal window, e.g. quiet 22:00–07:00 means send 07:00–22:00
            hour < start || hour >= end
        } else {
            // Inverted / all-day quiet (should not happen in practice)
            false
        }
    }
}
