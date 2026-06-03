use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::enums::{BillingInterval, SubscriptionInvoiceStatus};

// ── Feature keys ──────────────────────────────────────────────────────────────

/// Every capability in the platform. Naming convention: DOMAIN_CAPABILITY.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureKey {
    // ── Payments & Finance ──
    PaymentMpesaRecon,
    PaymentBulkDisbursements,
    AcctStandardLedger,
    AcctDoubleEntry,
    AcctKraCompliance,

    // ── Communications ──
    CommSmsAutomation,
    CommWhatsappAutomation,

    // ── Portals & Operations ──
    PortalResident,
    PortalCaretaker,
    PortalOwner,
    PortalVendor,

    // ── Maintenance & Utilities ──
    MaintWorkOrders,
    UtilityBilling,
    UtilityMeterReading,

    // ── Platform & Scale ──
    CoreMultiBranch,
    CoreCustomRoles,
    CoreAuditLog,
    BrandWhiteLabel,
}

impl FeatureKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PaymentMpesaRecon => "payment_mpesa_recon",
            Self::PaymentBulkDisbursements => "payment_bulk_disbursements",
            Self::AcctStandardLedger => "acct_standard_ledger",
            Self::AcctDoubleEntry => "acct_double_entry",
            Self::AcctKraCompliance => "acct_kra_compliance",
            Self::CommSmsAutomation => "comm_sms_automation",
            Self::CommWhatsappAutomation => "comm_whatsapp_automation",
            Self::PortalResident => "portal_resident",
            Self::PortalCaretaker => "portal_caretaker",
            Self::PortalOwner => "portal_owner",
            Self::PortalVendor => "portal_vendor",
            Self::MaintWorkOrders => "maint_work_orders",
            Self::UtilityBilling => "utility_billing",
            Self::UtilityMeterReading => "utility_meter_reading",
            Self::CoreMultiBranch => "core_multi_branch",
            Self::CoreCustomRoles => "core_custom_roles",
            Self::CoreAuditLog => "core_audit_log",
            Self::BrandWhiteLabel => "brand_white_label",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "payment_mpesa_recon" => Some(Self::PaymentMpesaRecon),
            "payment_bulk_disbursements" => Some(Self::PaymentBulkDisbursements),
            "acct_standard_ledger" => Some(Self::AcctStandardLedger),
            "acct_double_entry" => Some(Self::AcctDoubleEntry),
            "acct_kra_compliance" => Some(Self::AcctKraCompliance),
            "comm_sms_automation" => Some(Self::CommSmsAutomation),
            "comm_whatsapp_automation" => Some(Self::CommWhatsappAutomation),
            "portal_resident" => Some(Self::PortalResident),
            "portal_caretaker" => Some(Self::PortalCaretaker),
            "portal_owner" => Some(Self::PortalOwner),
            "portal_vendor" => Some(Self::PortalVendor),
            "maint_work_orders" => Some(Self::MaintWorkOrders),
            "utility_billing" => Some(Self::UtilityBilling),
            "utility_meter_reading" => Some(Self::UtilityMeterReading),
            "core_multi_branch" => Some(Self::CoreMultiBranch),
            "core_custom_roles" => Some(Self::CoreCustomRoles),
            "core_audit_log" => Some(Self::CoreAuditLog),
            "brand_white_label" => Some(Self::BrandWhiteLabel),
            _ => None,
        }
    }
}

impl fmt::Display for FeatureKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Limit keys ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitKey {
    MaxUnits,
    MaxBranches,
    MaxStorageMb,
    MaxSmsPerMonth,
    MaxWhatsappPerMonth,
}

impl LimitKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MaxUnits => "max_units",
            Self::MaxBranches => "max_branches",
            Self::MaxStorageMb => "max_storage_mb",
            Self::MaxSmsPerMonth => "max_sms_per_month",
            Self::MaxWhatsappPerMonth => "max_whatsapp_per_month",
        }
    }

    /// Redis hash field name for usage tracking (monthly counters use YYYY-MM suffix).
    pub fn usage_field(&self, period: Option<&str>) -> String {
        match period {
            Some(p) => format!("{}:{}", self.as_str(), p),
            None => self.as_str().to_string(),
        }
    }
}

impl fmt::Display for LimitKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Domain structs ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionPlan {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub price_kes: i32,
    pub yearly_price_kes: Option<i32>,
    pub interval: BillingInterval,
    pub is_active: bool,
    pub is_public: bool,
    pub sort_order: i32,
    pub trial_days: i32,
    pub metadata: Option<serde_json::Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// A single resolved feature — either from the plan or an agency override.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanFeature {
    pub feature_key: String,
    pub value_type: FeatureValueType,
    pub value: String,
    pub enabled: bool,
    pub source: FeatureSource, // "plan" | "override"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FeatureValueType {
    Boolean,
    Integer,
    String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FeatureSource {
    Plan,
    Override,
}

/// Fully resolved entitlements for an agency (post-override merge).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgencyEntitlements {
    pub features: HashMap<String, PlanFeature>,
    /// -1 = unlimited, 0 = not allowed
    pub limits: HashMap<String, i32>,
}

impl AgencyEntitlements {
    pub fn has_feature(&self, key: &FeatureKey) -> bool {
        self.features
            .get(key.as_str())
            .map(|f| f.enabled)
            .unwrap_or(false)
    }

    pub fn get_limit(&self, key: &LimitKey) -> i32 {
        self.limits.get(key.as_str()).copied().unwrap_or(0)
    }

    /// True when `current` is below the plan limit (or limit is unlimited).
    pub fn within_limit(&self, key: &LimitKey, current: i32) -> bool {
        let max = self.get_limit(key);
        max == -1 || current < max
    }

    /// True when `current` has reached or exceeded the soft-limit threshold.
    /// Soft limit: explicit `soft_limit` column, or 90 % of max.
    pub fn at_soft_limit(&self, key: &LimitKey, current: i32, soft_limit: Option<i32>) -> bool {
        let max = self.get_limit(key);
        if max == -1 {
            return false;
        }
        let threshold = soft_limit.unwrap_or_else(|| (max as f64 * 0.9) as i32);
        current >= threshold
    }
}

/// High-level subscription state (cheap — no feature map).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionState {
    pub is_active: bool,
    pub is_trial: bool,
    pub is_trial_expired: bool,
    pub is_paid: bool,
    pub in_grace_period: bool,
    pub plan_slug: String,
    pub plan_name: String,
    pub days_until_renewal: i64,
    pub trial_days_remaining: i64,
    /// "active" | "trialing" | "past_due" | "cancelled"
    pub status: String,
}

/// Result of a usage check (mirrors NestJS checkLimit return shape).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitCheckResult {
    pub allowed: bool,
    pub max: i32,
    pub current: i32,
    pub soft_limit_reached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionInvoice {
    pub id: Uuid,
    pub agency_subscription_id: Uuid,
    pub agency_id: Uuid,
    pub amount_kes: i32,
    pub status: SubscriptionInvoiceStatus,
    pub due_date: OffsetDateTime,
    pub paid_at: Option<OffsetDateTime>,
    pub mpesa_ref: Option<String>,
    pub mpesa_phone: Option<String>,
    pub receipt_url: Option<String>,
    pub notes: Option<String>,
    pub created_at: OffsetDateTime,
}

// ── Subscription events (broadcast to other subsystems) ───────────────────────

#[derive(Debug, Clone)]
pub enum SubscriptionEvent {
    Upgraded {
        agency_id: Uuid,
        plan_slug: String,
        mpesa_ref: Option<String>,
    },
    Cancelled {
        agency_id: Uuid,
        reason: Option<String>,
    },
    TrialExpired {
        agency_id: Uuid,
    },
    PastDue {
        agency_id: Uuid,
    },
}

#[derive(Debug, serde::Serialize)]
pub struct UsageSummary {
    pub branches: LimitCheckResult,
    pub units: LimitCheckResult,
    pub storage: LimitCheckResult,
    pub sms: LimitCheckResult,
    pub whatsapp: LimitCheckResult,
}
