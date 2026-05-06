use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::subscription::{
        AgencyEntitlements, LimitCheckResult, SubscriptionInvoice, SubscriptionPlan,
        SubscriptionState,
    },
};

#[async_trait]
pub trait SubscriptionRepository: Send + Sync + 'static {
    // ── State & Entitlements ─────────────────────────────────────────────────

    /// Full entitlements map (features + limits) merged with per-agency overrides.
    /// Returns `None` when no active/trialing subscription exists.
    async fn get_entitlements(
        &self,
        agency_id: Uuid,
    ) -> Result<Option<AgencyEntitlements>, AppError>;

    /// Lightweight subscription state — no feature map.
    async fn get_state(&self, agency_id: Uuid) -> Result<Option<SubscriptionState>, AppError>;

    // ── Plan catalogue ───────────────────────────────────────────────────────

    async fn list_plans(&self) -> Result<Vec<SubscriptionPlan>, AppError>;

    async fn get_plan_by_slug(&self, slug: &str) -> Result<Option<SubscriptionPlan>, AppError>;

    // ── Lifecycle ────────────────────────────────────────────────────────────

    /// Initial subscribe OR plan change. Upserts the subscription row.
    async fn upsert_subscription(
        &self,
        agency_id: Uuid,
        plan_slug: &str,
        status: &str, // "active" | "trialing"
        mpesa_ref: Option<&str>,
        custom_price: Option<i32>,
    ) -> Result<(), AppError>;

    async fn cancel_subscription(
        &self,
        agency_id: Uuid,
        reason: Option<&str>,
    ) -> Result<(), AppError>;

    // ── Usage counting (DB side) ─────────────────────────────────────────────

    /// Count rows in a agency-schema table (properties, units, etc.).
    async fn count_tenant_rows(&self, agency_id: Uuid, table_name: &str) -> Result<i32, AppError>;

    /// Count rows in the platform schema filtered by agency_id.
    async fn count_platform_rows(&self, agency_id: Uuid, table_name: &str)
        -> Result<i32, AppError>;

    /// Check limit against plan; returns LimitCheckResult (with soft_limit).
    async fn check_limit_with_result(
        &self,
        agency_id: Uuid,
        limit_key: &str,
        current_count: i32,
    ) -> Result<LimitCheckResult, AppError>;

    // ── Admin overrides ──────────────────────────────────────────────────────

    async fn set_feature_override(
        &self,
        agency_id: Uuid,
        feature_key: &str,
        value: &str,
        reason: Option<&str>,
        created_by: Option<Uuid>,
        expires_at: Option<OffsetDateTime>,
    ) -> Result<(), AppError>;

    /// Remove a feature override row entirely (vs. setting value = 'false').
    async fn remove_feature_override(
        &self,
        agency_id: Uuid,
        feature_key: &str,
    ) -> Result<(), AppError>;

    // ── Invoices ─────────────────────────────────────────────────────────────

    async fn list_invoices(
        &self,
        agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<SubscriptionInvoice>, AppError>;

    async fn record_payment(
        &self,
        agency_id: Uuid,
        amount_kes: i32,
        mpesa_ref: &str,
        mpesa_phone: Option<&str>,
    ) -> Result<Uuid, AppError>;

    // ── M-Pesa STK Push pending requests ─────────────────────────────────────

    /// Persist an in-flight STK Push initiated for a subscription payment.
    /// Called immediately after a successful `stk_push()` API call.
    async fn save_pending_mpesa_request(
        &self,
        agency_id: Uuid,
        plan_slug: &str,
        amount_kes: i32,
        checkout_request_id: &str,
        merchant_request_id: &str,
    ) -> Result<(), AppError>;

    /// Look up and atomically delete a pending request by `checkout_request_id`.
    ///
    /// Returns `Some((agency_id, plan_slug))` if a non-expired row was found,
    /// or `None` if the request doesn't exist, was already settled, or expired.
    /// Called from the M-Pesa webhook callback on a successful result code.
    async fn settle_pending_mpesa_request(
        &self,
        checkout_request_id: &str,
    ) -> Result<Option<(Uuid, String)>, AppError>;
}
