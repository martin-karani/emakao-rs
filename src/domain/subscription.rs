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
    // Core
    CoreMultiBranch,
    CoreRolesRbac,
    CoreAuditLog,
    CoreTwoFactor,
    CoreApiAccess,
    // Payments
    PaymentMpesaStk,
    PaymentMpesaPaybill,
    PaymentMpesaTill,
    PaymentBankTransfer,
    PaymentAutoReceipts,
    PaymentReminders,
    PaymentLateFees,
    // Owner portal
    OwnerPortal,
    OwnerStatements,
    OwnerDocumentSharing,
    OwnerDisbursements,
    // Lease
    LeaseDigitalSigning,
    LeaseLeaseTemplates,
    LeaseEscalationRules,
    LeaseMultiCurrency,
    // Maintenance
    MaintWorkOrders,
    MaintVendorPortal,
    MaintPreventiveSched,
    MaintAssetRegister,
    MaintAmenityBookings,
    // Communications
    CommSmsBasic,
    CommSmsBranded,
    CommWhatsapp,
    CommEmailCampaigns,
    CommTenantPortal,
    CommTenantApp,
    // Accounting
    AcctDoubleEntry,
    AcctVatReports,
    AcctManagementFee,
    AcctExpenseTracking,
    AcctBankReconciliation,
    AcctQuickbooksSync,
    // Reporting
    ReportOccupancy,
    ReportCashflow,
    ReportArrearsAging,
    ReportCustomBuilder,
    ReportScheduledExport,
    ReportPortfolioSummary,
    // Marketing
    MktgListingsPage,
    MktgWebsiteEmbed,
    MktgLeadCapture,
    MktgVirtualTours,
    // Branding
    BrandWhiteLabel,
    BrandCustomDomain,
    BrandCustomEmailDomain,
    // Integrations
    IntegUtilityBilling,
    IntegCreditCheck,
    IntegCountyPermits,
    IntegWebhooks,
    IntegZapier,
}

impl FeatureKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CoreMultiBranch => "core_multi_branch",
            Self::CoreRolesRbac => "core_roles_rbac",
            Self::CoreAuditLog => "core_audit_log",
            Self::CoreTwoFactor => "core_two_factor",
            Self::CoreApiAccess => "core_api_access",
            Self::PaymentMpesaStk => "payment_mpesa_stk",
            Self::PaymentMpesaPaybill => "payment_mpesa_paybill",
            Self::PaymentMpesaTill => "payment_mpesa_till",
            Self::PaymentBankTransfer => "payment_bank_transfer",
            Self::PaymentAutoReceipts => "payment_auto_receipts",
            Self::PaymentReminders => "payment_reminders",
            Self::PaymentLateFees => "payment_late_fees",
            Self::OwnerPortal => "owner_portal",
            Self::OwnerStatements => "owner_statements",
            Self::OwnerDocumentSharing => "owner_document_sharing",
            Self::OwnerDisbursements => "owner_disbursements",
            Self::LeaseDigitalSigning => "lease_digital_signing",
            Self::LeaseLeaseTemplates => "lease_lease_templates",
            Self::LeaseEscalationRules => "lease_escalation_rules",
            Self::LeaseMultiCurrency => "lease_multi_currency",
            Self::MaintWorkOrders => "maint_work_orders",
            Self::MaintVendorPortal => "maint_vendor_portal",
            Self::MaintPreventiveSched => "maint_preventive_sched",
            Self::MaintAssetRegister => "maint_asset_register",
            Self::MaintAmenityBookings => "maint_amenity_bookings",
            Self::CommSmsBasic => "comm_sms_basic",
            Self::CommSmsBranded => "comm_sms_branded",
            Self::CommWhatsapp => "comm_whatsapp",
            Self::CommEmailCampaigns => "comm_email_campaigns",
            Self::CommTenantPortal => "comm_tenant_portal",
            Self::CommTenantApp => "comm_tenant_app",
            Self::AcctDoubleEntry => "acct_double_entry",
            Self::AcctVatReports => "acct_vat_reports",
            Self::AcctManagementFee => "acct_management_fee",
            Self::AcctExpenseTracking => "acct_expense_tracking",
            Self::AcctBankReconciliation => "acct_bank_reconciliation",
            Self::AcctQuickbooksSync => "acct_quickbooks_sync",
            Self::ReportOccupancy => "report_occupancy",
            Self::ReportCashflow => "report_cashflow",
            Self::ReportArrearsAging => "report_arrears_aging",
            Self::ReportCustomBuilder => "report_custom_builder",
            Self::ReportScheduledExport => "report_scheduled_export",
            Self::ReportPortfolioSummary => "report_portfolio_summary",
            Self::MktgListingsPage => "mktg_listings_page",
            Self::MktgWebsiteEmbed => "mktg_website_embed",
            Self::MktgLeadCapture => "mktg_lead_capture",
            Self::MktgVirtualTours => "mktg_virtual_tours",
            Self::BrandWhiteLabel => "brand_white_label",
            Self::BrandCustomDomain => "brand_custom_domain",
            Self::BrandCustomEmailDomain => "brand_custom_email_domain",
            Self::IntegUtilityBilling => "integ_utility_billing",
            Self::IntegCreditCheck => "integ_credit_check",
            Self::IntegCountyPermits => "integ_county_permits",
            Self::IntegWebhooks => "integ_webhooks",
            Self::IntegZapier => "integ_zapier",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "core_multi_branch" => Some(Self::CoreMultiBranch),
            "core_roles_rbac" => Some(Self::CoreRolesRbac),
            "core_audit_log" => Some(Self::CoreAuditLog),
            "core_two_factor" => Some(Self::CoreTwoFactor),
            "core_api_access" => Some(Self::CoreApiAccess),
            "payment_mpesa_stk" => Some(Self::PaymentMpesaStk),
            "payment_mpesa_paybill" => Some(Self::PaymentMpesaPaybill),
            "payment_mpesa_till" => Some(Self::PaymentMpesaTill),
            "payment_bank_transfer" => Some(Self::PaymentBankTransfer),
            "payment_auto_receipts" => Some(Self::PaymentAutoReceipts),
            "payment_reminders" => Some(Self::PaymentReminders),
            "payment_late_fees" => Some(Self::PaymentLateFees),
            "owner_portal" => Some(Self::OwnerPortal),
            "owner_statements" => Some(Self::OwnerStatements),
            "owner_document_sharing" => Some(Self::OwnerDocumentSharing),
            "owner_disbursements" => Some(Self::OwnerDisbursements),
            "lease_digital_signing" => Some(Self::LeaseDigitalSigning),
            "lease_lease_templates" => Some(Self::LeaseLeaseTemplates),
            "lease_escalation_rules" => Some(Self::LeaseEscalationRules),
            "lease_multi_currency" => Some(Self::LeaseMultiCurrency),
            "maint_work_orders" => Some(Self::MaintWorkOrders),
            "maint_vendor_portal" => Some(Self::MaintVendorPortal),
            "maint_preventive_sched" => Some(Self::MaintPreventiveSched),
            "maint_asset_register" => Some(Self::MaintAssetRegister),
            "maint_amenity_bookings" => Some(Self::MaintAmenityBookings),
            "comm_sms_basic" => Some(Self::CommSmsBasic),
            "comm_sms_branded" => Some(Self::CommSmsBranded),
            "comm_whatsapp" => Some(Self::CommWhatsapp),
            "comm_email_campaigns" => Some(Self::CommEmailCampaigns),
            "comm_tenant_portal" => Some(Self::CommTenantPortal),
            "comm_tenant_app" => Some(Self::CommTenantApp),
            "acct_double_entry" => Some(Self::AcctDoubleEntry),
            "acct_vat_reports" => Some(Self::AcctVatReports),
            "acct_management_fee" => Some(Self::AcctManagementFee),
            "acct_expense_tracking" => Some(Self::AcctExpenseTracking),
            "acct_bank_reconciliation" => Some(Self::AcctBankReconciliation),
            "acct_quickbooks_sync" => Some(Self::AcctQuickbooksSync),
            "report_occupancy" => Some(Self::ReportOccupancy),
            "report_cashflow" => Some(Self::ReportCashflow),
            "report_arrears_aging" => Some(Self::ReportArrearsAging),
            "report_custom_builder" => Some(Self::ReportCustomBuilder),
            "report_scheduled_export" => Some(Self::ReportScheduledExport),
            "report_portfolio_summary" => Some(Self::ReportPortfolioSummary),
            "mktg_listings_page" => Some(Self::MktgListingsPage),
            "mktg_website_embed" => Some(Self::MktgWebsiteEmbed),
            "mktg_lead_capture" => Some(Self::MktgLeadCapture),
            "mktg_virtual_tours" => Some(Self::MktgVirtualTours),
            "brand_white_label" => Some(Self::BrandWhiteLabel),
            "brand_custom_domain" => Some(Self::BrandCustomDomain),
            "brand_custom_email_domain" => Some(Self::BrandCustomEmailDomain),
            "integ_utility_billing" => Some(Self::IntegUtilityBilling),
            "integ_credit_check" => Some(Self::IntegCreditCheck),
            "integ_county_permits" => Some(Self::IntegCountyPermits),
            "integ_webhooks" => Some(Self::IntegWebhooks),
            "integ_zapier" => Some(Self::IntegZapier),
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
    MaxUsers,
    MaxProperties,
    MaxUnits,
    MaxOwners,
    MaxBranches,
    MaxStorageMb,
    MaxSmsPerMonth,
    MaxWhatsappPerMonth,
    MaxApiCallsPerDay,
    MaxCustomReports,
    MaxDocumentTemplates,
    MaxApplicants,
    MaxVendors,
}

impl LimitKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MaxUsers => "max_users",
            Self::MaxProperties => "max_properties",
            Self::MaxUnits => "max_units",
            Self::MaxOwners => "max_owners",
            Self::MaxBranches => "max_branches",
            Self::MaxStorageMb => "max_storage_mb",
            Self::MaxSmsPerMonth => "max_sms_per_month",
            Self::MaxWhatsappPerMonth => "max_whatsapp_per_month",
            Self::MaxApiCallsPerDay => "max_api_calls_per_day",
            Self::MaxCustomReports => "max_custom_reports",
            Self::MaxDocumentTemplates => "max_document_templates",
            Self::MaxApplicants => "max_applicants",
            Self::MaxVendors => "max_vendors",
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
    pub properties: LimitCheckResult,
    pub units: LimitCheckResult,
    pub users: LimitCheckResult,
    pub storage: LimitCheckResult,
    pub sms: LimitCheckResult,
    pub whatsapp: LimitCheckResult,
    pub api_calls: LimitCheckResult,
}
