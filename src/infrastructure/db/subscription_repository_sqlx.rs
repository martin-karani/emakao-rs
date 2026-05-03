use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::subscription_repository::SubscriptionRepository},
    domain::subscription::{
        AgencyEntitlements, FeatureSource, FeatureValueType, InvoiceStatus, LimitCheckResult,
        PlanFeature, PlanInterval, SubscriptionInvoice, SubscriptionPlan, SubscriptionState,
    },
};

// ── Private row types ─────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct FeatureRow {
    feature_key: String,
    value_type: String,
    value: String,
    enabled: bool,
    is_override: bool,
}

#[derive(sqlx::FromRow)]
struct LimitRow {
    limit_key: String,
    max_value: i32,
}

#[derive(sqlx::FromRow)]
struct StateRow {
    status: String,
    plan_slug: String,
    plan_name: String,
    trial_ends_at: Option<OffsetDateTime>,
    current_period_end: Option<OffsetDateTime>,
    grace_period_ends_at: Option<OffsetDateTime>,
}

#[derive(sqlx::FromRow)]
struct PlanRow {
    id: Uuid,
    slug: String,
    name: String,
    description: Option<String>,
    price_kes: i32,
    yearly_price_kes: Option<i32>,
    interval: String,
    is_active: bool,
    is_public: bool,
    sort_order: i32,
    trial_days: i32,
    metadata: Option<serde_json::Value>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct InvoiceRow {
    id: Uuid,
    agency_subscription_id: Uuid,
    agency_id: Uuid,
    amount_kes: i32,
    status: String,
    due_date: OffsetDateTime,
    paid_at: Option<OffsetDateTime>,
    mpesa_ref: Option<String>,
    mpesa_phone: Option<String>,
    receipt_url: Option<String>,
    notes: Option<String>,
    created_at: OffsetDateTime,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_interval(s: &str) -> PlanInterval {
    match s {
        "yearly" => PlanInterval::Yearly,
        _ => PlanInterval::Monthly,
    }
}

fn parse_value_type(s: &str) -> FeatureValueType {
    match s {
        "integer" => FeatureValueType::Integer,
        "string" => FeatureValueType::String,
        _ => FeatureValueType::Boolean,
    }
}

fn parse_invoice_status(s: &str) -> InvoiceStatus {
    match s {
        "open" => InvoiceStatus::Open,
        "paid" => InvoiceStatus::Paid,
        "void" => InvoiceStatus::Void,
        "uncollectible" => InvoiceStatus::Uncollectible,
        _ => InvoiceStatus::Draft,
    }
}

impl From<PlanRow> for SubscriptionPlan {
    fn from(r: PlanRow) -> Self {
        Self {
            id: r.id,
            slug: r.slug,
            name: r.name,
            description: r.description,
            price_kes: r.price_kes,
            yearly_price_kes: r.yearly_price_kes,
            interval: parse_interval(&r.interval),
            is_active: r.is_active,
            is_public: r.is_public,
            sort_order: r.sort_order,
            trial_days: r.trial_days,
            metadata: r.metadata,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ── Repo struct ───────────────────────────────────────────────────────────────

/// The closure must be async because `TenantPoolManager::for_agency` is async
/// (it may hit the DB to resolve agency_id → schema_name on a cold cache).
/// On subsequent calls for the same agency the pool is returned from the
/// in-memory `DashMap` with zero extra allocations.
type TenantPoolFn =
    Box<dyn Fn(Uuid) -> Pin<Box<dyn Future<Output = anyhow::Result<PgPool>> + Send>> + Send + Sync>;

pub struct PgSubscriptionRepo {
    platform: PgPool,
    tenant_pool_fn: TenantPoolFn,
}

impl PgSubscriptionRepo {
    /// * `platform`       — platform (public schema) pool.
    /// * `tenant_pool_fn` — async closure: given an agency UUID returns the
    ///   schema-scoped pool for that tenant.
    ///
    /// Typical wiring in `AppState::build`:
    /// ```rust
    /// let pools = tenant_pools.clone();
    /// PgSubscriptionRepo::new(platform_pool.clone(), move |agency_id| {
    ///     let pools = pools.clone();
    ///     Box::pin(async move { pools.for_agency(agency_id).await })
    /// })
    /// ```
    pub fn new(
        platform: PgPool,
        tenant_pool_fn: impl Fn(Uuid) -> Pin<Box<dyn Future<Output = anyhow::Result<PgPool>> + Send>>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self {
            platform,
            tenant_pool_fn: Box::new(tenant_pool_fn),
        }
    }

    /// Resolves the tenant pool for `agency_id` asynchronously.
    async fn tenant_pool(&self, agency_id: Uuid) -> Result<PgPool, AppError> {
        (self.tenant_pool_fn)(agency_id)
            .await
            .map_err(|e| AppError::InternalServer(e.to_string()))
    }
}

#[async_trait]
impl SubscriptionRepository for PgSubscriptionRepo {
    async fn get_entitlements(
        &self,
        agency_id: Uuid,
    ) -> Result<Option<AgencyEntitlements>, AppError> {
        let feature_rows: Vec<FeatureRow> = sqlx::query_as!(
            FeatureRow,
            r#"
            SELECT
                COALESCE(fo.feature_key, pf.feature_key)  AS "feature_key!",
                COALESCE(pf.value_type, 'boolean')        AS "value_type!",
                COALESCE(fo.value, pf.value)              AS "value!",
                CASE
                    WHEN fo.id IS NOT NULL THEN (fo.value = 'true')
                    ELSE pf.enabled
                END                                       AS "enabled!",
                (fo.id IS NOT NULL)                       AS "is_override!"
            FROM subscriptions s
            JOIN subscription_plans sp  ON sp.id = s.plan_id
            JOIN plan_features      pf  ON pf.plan_id = sp.id
            LEFT JOIN feature_overrides fo
                   ON fo.agency_id    = s.agency_id
                  AND fo.feature_key  = pf.feature_key
                  AND (fo.expires_at IS NULL OR fo.expires_at > now())
            WHERE s.agency_id = $1
              AND s.status IN ('active', 'trialing')
            "#,
            agency_id
        )
        .fetch_all(&self.platform)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        if feature_rows.is_empty() {
            return Ok(None);
        }

        let limit_rows: Vec<LimitRow> = sqlx::query_as!(
            LimitRow,
            r#"
            SELECT pl.limit_key  AS "limit_key!",
                   pl.max_value  AS "max_value!"
            FROM subscriptions s
            JOIN subscription_plans sp ON sp.id = s.plan_id
            JOIN plan_limits        pl ON pl.plan_id = sp.id
            WHERE s.agency_id = $1
              AND s.status IN ('active', 'trialing')
            "#,
            agency_id
        )
        .fetch_all(&self.platform)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        let features = feature_rows
            .into_iter()
            .map(|r| {
                let source = if r.is_override {
                    FeatureSource::Override
                } else {
                    FeatureSource::Plan
                };
                (
                    r.feature_key.clone(),
                    PlanFeature {
                        feature_key: r.feature_key,
                        value_type: parse_value_type(&r.value_type),
                        value: r.value,
                        enabled: r.enabled,
                        source,
                    },
                )
            })
            .collect();

        let limits = limit_rows
            .into_iter()
            .map(|r| (r.limit_key, r.max_value))
            .collect();

        Ok(Some(AgencyEntitlements { features, limits }))
    }

    // ── State ────────────────────────────────────────────────────────────────

    async fn get_state(&self, agency_id: Uuid) -> Result<Option<SubscriptionState>, AppError> {
        let row = sqlx::query_as!(
            StateRow,
            r#"
            SELECT s.status::TEXT              AS "status!",
                   sp.slug              AS "plan_slug!",
                   sp.name              AS "plan_name!",
                   s.trial_ends_at,
                   s.current_period_end,
                   s.grace_period_ends_at
            FROM subscriptions s
            JOIN subscription_plans sp ON sp.id = s.plan_id
            WHERE s.agency_id = $1
            ORDER BY s.created_at DESC
            LIMIT 1
            "#,
            agency_id
        )
        .fetch_optional(&self.platform)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        let Some(r) = row else { return Ok(None) };

        let now = OffsetDateTime::now_utc();
        let is_trial = r.status == "trialing";
        let is_trial_expired = is_trial && r.trial_ends_at.map(|t| t < now).unwrap_or(false);
        let in_grace = r.grace_period_ends_at.map(|g| g > now).unwrap_or(false);
        let is_active = matches!(r.status.as_str(), "active" | "trialing") && !is_trial_expired;
        let is_paid = r.status == "active";

        let days_until_renewal = r
            .current_period_end
            .map(|t| (t - now).whole_days().max(0))
            .unwrap_or(0);
        let trial_days_remaining = if is_trial && !is_trial_expired {
            r.trial_ends_at
                .map(|t| (t - now).whole_days().max(0))
                .unwrap_or(0)
        } else {
            0
        };

        Ok(Some(SubscriptionState {
            is_active,
            is_trial,
            is_trial_expired,
            is_paid,
            in_grace_period: in_grace,
            plan_slug: r.plan_slug,
            plan_name: r.plan_name,
            days_until_renewal,
            trial_days_remaining,
            status: r.status,
        }))
    }

    // ── Plans ────────────────────────────────────────────────────────────────

    async fn list_plans(&self) -> Result<Vec<SubscriptionPlan>, AppError> {
        let rows: Vec<PlanRow> = sqlx::query_as!(
            PlanRow,
            r#"
            SELECT id, slug, name, description, price_kes, yearly_price_kes,
                   interval, is_active, is_public, sort_order, trial_days,
                   metadata, created_at, updated_at
            FROM subscription_plans
            WHERE is_active = true AND is_public = true
            ORDER BY sort_order ASC
            "#
        )
        .fetch_all(&self.platform)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(rows.into_iter().map(SubscriptionPlan::from).collect())
    }

    async fn get_plan_by_slug(&self, slug: &str) -> Result<Option<SubscriptionPlan>, AppError> {
        let row = sqlx::query_as!(
            PlanRow,
            r#"
            SELECT id, slug, name, description, price_kes, yearly_price_kes,
                   interval, is_active, is_public, sort_order, trial_days,
                   metadata, created_at, updated_at
            FROM subscription_plans WHERE slug = $1
            "#,
            slug
        )
        .fetch_optional(&self.platform)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(row.map(SubscriptionPlan::from))
    }

    // ── Lifecycle ────────────────────────────────────────────────────────────

    async fn upsert_subscription(
        &self,
        agency_id: Uuid,
        plan_slug: &str,
        status: &str,
        mpesa_ref: Option<&str>,
        custom_price: Option<i32>,
    ) -> Result<(), AppError> {
        // Double-cast $3::text::subscription_status:
        //   - ::text  tells sqlx the parameter is a plain string (has a built-in mapping)
        //   - ::subscription_status  tells Postgres to cast that text to the enum
        // Using $3::subscription_status alone fails because sqlx has no Rust↔enum mapping.
        sqlx::query!(
            r#"
            UPDATE subscriptions s
            SET plan_id               = sp.id,
                status                = $3::text::subscription_status,
                current_period_start  = now(),
                current_period_end    = now() + INTERVAL '1 month',
                last_payment_ref      = $4,
                custom_price_kes      = $5,
                payment_failure_count = 0,
                updated_at            = now()
            FROM subscription_plans sp
            WHERE sp.slug     = $2
              AND s.agency_id = $1
            "#,
            agency_id,
            plan_slug,
            status,
            mpesa_ref,
            custom_price
        )
        .execute(&self.platform)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        sqlx::query!(
            r#"
            UPDATE subscriptions
            SET plan_tier = CASE $1::text
                WHEN 'starter'    THEN 'core'
                WHEN 'growth'     THEN 'plus'
                WHEN 'enterprise' THEN 'max'
                ELSE plan_tier
            END
            WHERE agency_id = $2::uuid
            "#,
            plan_slug,
            agency_id
        )
        .execute(&self.platform)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(())
    }

    async fn cancel_subscription(
        &self,
        agency_id: Uuid,
        reason: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            UPDATE subscriptions
            SET status               = 'cancelled',
                cancelled_at         = now(),
                cancel_reason        = $2,
                grace_period_ends_at = current_period_end,
                updated_at           = now()
            WHERE agency_id = $1
              AND status NOT IN ('cancelled')
            "#,
            agency_id,
            reason
        )
        .execute(&self.platform)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(())
    }

    // ── Usage counting ───────────────────────────────────────────────────────

    /// Counts rows in a **tenant-schema** table.
    /// `tenant_pool()` is async: on the first call for a given agency it
    /// resolves agency_id → schema_name via a single platform DB query, then
    /// caches the pool in `TenantPoolManager`'s DashMap for all future calls.
    async fn count_tenant_rows(&self, agency_id: Uuid, table_name: &str) -> Result<i32, AppError> {
        let pool = self.tenant_pool(agency_id).await?;
        let row = sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {table_name}"))
            .fetch_one(&pool)
            .await
            .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(row as i32)
    }

    async fn count_platform_rows(
        &self,
        agency_id: Uuid,
        table_name: &str,
    ) -> Result<i32, AppError> {
        let row = sqlx::query_scalar::<_, i64>(&format!(
            "SELECT COUNT(*) FROM {table_name} WHERE agency_id = $1"
        ))
        .bind(agency_id)
        .fetch_one(&self.platform)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(row as i32)
    }

    async fn check_limit_with_result(
        &self,
        agency_id: Uuid,
        limit_key: &str,
        current_count: i32,
    ) -> Result<LimitCheckResult, AppError> {
        let row = sqlx::query!(
            r#"
            SELECT pl.max_value, pl.soft_limit
            FROM subscriptions s
            JOIN subscription_plans sp ON sp.id = s.plan_id
            JOIN plan_limits        pl ON pl.plan_id = sp.id
            WHERE s.agency_id = $1
              AND pl.limit_key = $2
              AND s.status IN ('active', 'trialing')
            "#,
            agency_id,
            limit_key
        )
        .fetch_optional(&self.platform)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        let (max, soft) = row
            .map(|r| (r.max_value, r.soft_limit))
            .unwrap_or((0, None));
        let threshold = soft.unwrap_or_else(|| (max as f64 * 0.9) as i32);

        Ok(LimitCheckResult {
            allowed: max == -1 || current_count < max,
            max,
            current: current_count,
            soft_limit_reached: max != -1 && current_count >= threshold,
        })
    }

    // ── Overrides ────────────────────────────────────────────────────────────

    async fn set_feature_override(
        &self,
        agency_id: Uuid,
        feature_key: &str,
        value: &str,
        reason: Option<&str>,
        created_by: Option<Uuid>,
        expires_at: Option<OffsetDateTime>,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO feature_overrides
                (agency_id, feature_key, value, reason, created_by, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (agency_id, feature_key) DO UPDATE
                SET value      = EXCLUDED.value,
                    reason     = EXCLUDED.reason,
                    created_by = EXCLUDED.created_by,
                    expires_at = EXCLUDED.expires_at
            "#,
            agency_id,
            feature_key,
            value,
            reason,
            created_by,
            expires_at
        )
        .execute(&self.platform)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;
        Ok(())
    }

    async fn remove_feature_override(
        &self,
        agency_id: Uuid,
        feature_key: &str,
    ) -> Result<(), AppError> {
        sqlx::query!(
            "DELETE FROM feature_overrides WHERE agency_id = $1 AND feature_key = $2",
            agency_id,
            feature_key
        )
        .execute(&self.platform)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;
        Ok(())
    }

    // ── Invoices ─────────────────────────────────────────────────────────────

    async fn list_invoices(
        &self,
        agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<SubscriptionInvoice>, AppError> {
        let rows: Vec<InvoiceRow> = sqlx::query_as!(
            InvoiceRow,
            r#"
            SELECT id, agency_subscription_id, agency_id, amount_kes, status,
                   due_date, paid_at, mpesa_ref, mpesa_phone, receipt_url,
                   notes, created_at
            FROM subscription_invoices
            WHERE agency_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            agency_id,
            limit,
            offset
        )
        .fetch_all(&self.platform)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| SubscriptionInvoice {
                id: r.id,
                agency_subscription_id: r.agency_subscription_id,
                agency_id: r.agency_id,
                amount_kes: r.amount_kes,
                status: parse_invoice_status(&r.status),
                due_date: r.due_date,
                paid_at: r.paid_at,
                mpesa_ref: r.mpesa_ref,
                mpesa_phone: r.mpesa_phone,
                receipt_url: r.receipt_url,
                notes: r.notes,
                created_at: r.created_at,
            })
            .collect())
    }

    async fn record_payment(
        &self,
        agency_id: Uuid,
        amount_kes: i32,
        mpesa_ref: &str,
        mpesa_phone: Option<&str>,
    ) -> Result<Uuid, AppError> {
        let sub_id: Uuid = sqlx::query_scalar!(
            "SELECT id FROM subscriptions WHERE agency_id = $1 ORDER BY created_at DESC LIMIT 1",
            agency_id
        )
        .fetch_one(&self.platform)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        let invoice_id: Uuid = sqlx::query_scalar!(
            r#"
            INSERT INTO subscription_invoices
                (agency_subscription_id, agency_id, amount_kes, status, due_date, paid_at,
                 mpesa_ref, mpesa_phone)
            VALUES ($1, $2, $3, 'paid', now(), now(), $4, $5)
            RETURNING id
            "#,
            sub_id,
            agency_id,
            amount_kes,
            mpesa_ref,
            mpesa_phone
        )
        .fetch_one(&self.platform)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        sqlx::query!(
            r#"
            UPDATE subscriptions
            SET status                = 'active',
                last_payment_ref      = $2,
                last_payment_at       = now(),
                current_period_end    = now() + INTERVAL '1 month',
                payment_failure_count = 0,
                updated_at            = now()
            WHERE agency_id = $1
              AND status IN ('past_due', 'trialing', 'active')
            "#,
            agency_id,
            mpesa_ref
        )
        .execute(&self.platform)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(invoice_id)
    }

    // ── M-Pesa STK Push pending requests ─────────────────────────────────────

    async fn save_pending_mpesa_request(
        &self,
        agency_id: Uuid,
        plan_slug: &str,
        amount_kes: i32,
        checkout_request_id: &str,
        merchant_request_id: &str,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO pending_mpesa_subscription_requests
                (agency_id, plan_slug, amount_kes, checkout_request_id, merchant_request_id)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (checkout_request_id) DO NOTHING
            "#,
            agency_id,
            plan_slug,
            amount_kes,
            checkout_request_id,
            merchant_request_id,
        )
        .execute(&self.platform)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(())
    }

    async fn settle_pending_mpesa_request(
        &self,
        checkout_request_id: &str,
    ) -> Result<Option<(Uuid, String)>, AppError> {
        let row = sqlx::query!(
            r#"
            DELETE FROM pending_mpesa_subscription_requests
            WHERE checkout_request_id = $1
              AND expires_at > now()
            RETURNING agency_id, plan_slug
            "#,
            checkout_request_id,
        )
        .fetch_optional(&self.platform)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(row.map(|r| (r.agency_id, r.plan_slug)))
    }
}
