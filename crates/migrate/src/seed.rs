//
// Platform seed data.  Lives here so the `migrate` binary is fully
// self-contained: no dependency on the main `emakao` crate, no compile-time
// SQL macros.  Every query uses plain `sqlx::query(...)` + `.bind()`.
//
// All functions are idempotent — safe to call on every `make migrate` run.
// Plans use the table's DEFAULT `uuid_generate_v4()` for their primary key so
// the data is portable across fresh databases without hard-coded UUIDs.

use anyhow::{Context, Result};
use sqlx::PgPool;

// ─────────────────────────────────────────────────────────────────────────────
// Public entry point
// ─────────────────────────────────────────────────────────────────────────────

/// Seed everything the platform DB needs before the first agency is created.
/// Must be called a    fter platform migrations have run successfully.
pub async fn run_platform_seeds(pool: &PgPool) -> Result<()> {
    seed_subscription_plans(pool).await?;
    seed_plan_features(pool).await?;
    seed_plan_limits(pool).await?;
    Ok(())
}

struct PlanSeed {
    slug: &'static str,
    name: &'static str,
    description: &'static str,
    price_kes: i32,
    yearly_price_kes: i32,
    trial_days: i32,
    sort_order: i32,
}

async fn seed_subscription_plans(pool: &PgPool) -> Result<()> {
    const PLANS: &[PlanSeed] = &[
        PlanSeed {
            slug: "professional",
            name: "Professional Tier",
            description: "Essential workflow organization and manual financial tracking for growing portfolios.",
            price_kes: 5_000,
            yearly_price_kes: 50_000,
            trial_days: 14,
            sort_order: 1,
        },
        PlanSeed {
            slug: "enterprise",
            name: "Enterprise Compliance Engine",
            description: "Automated eTIMS generation, eRITS exports, WHT compliance, and direct M-Pesa automated reconciliation.",
            price_kes: 19_500,
            yearly_price_kes: 195_000,
            trial_days: 14,
            sort_order: 2,
        },
    ];

    for p in PLANS {
        sqlx::query(
            r#"
            INSERT INTO subscription_plans 
                (slug, name, description, price_kes, yearly_price_kes, "interval", trial_days, is_active, is_public, sort_order)
            VALUES ($1, $2, $3, $4, $5, 'monthly', $6, true, true, $7)
            ON CONFLICT (slug) DO UPDATE SET
                name = EXCLUDED.name, description = EXCLUDED.description,
                price_kes = EXCLUDED.price_kes, yearly_price_kes = EXCLUDED.yearly_price_kes,
                sort_order = EXCLUDED.sort_order, updated_at = now()
            "#
        )
        .bind(p.slug).bind(p.name).bind(p.description).bind(p.price_kes).bind(p.yearly_price_kes).bind(p.trial_days).bind(p.sort_order)
        .execute(pool).await.context("Failed to seed subscription plans")?;
    }
    Ok(())
}

struct FeatureSeed {
    plan: &'static str,
    key: &'static str,
    enabled: bool,
}

async fn seed_plan_features(pool: &PgPool) -> Result<()> {
    const FEATURES: &[FeatureSeed] = &[
        // Professional Tier Core Features
        FeatureSeed {
            plan: "professional",
            key: "portal_resident",
            enabled: true,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "portal_resident",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "portal_caretaker",
            enabled: true,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "portal_caretaker",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "portal_owner",
            enabled: true,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "portal_owner",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "maint_work_orders",
            enabled: true,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "maint_work_orders",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "utility_billing",
            enabled: true,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "utility_billing",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "utility_meter_reading",
            enabled: true,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "utility_meter_reading",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "payment_mpesa_recon",
            enabled: true,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "payment_mpesa_recon",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "acct_standard_ledger",
            enabled: true,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "acct_standard_ledger",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "comm_sms_automation",
            enabled: true,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "comm_sms_automation",
            enabled: true,
        },
        // Enterprise Compliance & Automation Suite
        FeatureSeed {
            plan: "professional",
            key: "payment_bulk_disbursements",
            enabled: false,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "payment_bulk_disbursements",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "acct_double_entry",
            enabled: false,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "acct_double_entry",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "acct_kra_compliance",
            enabled: false,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "acct_kra_compliance",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "comm_whatsapp_automation",
            enabled: false,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "comm_whatsapp_automation",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "portal_vendor",
            enabled: false,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "portal_vendor",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "core_multi_branch",
            enabled: true,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "core_multi_branch",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "core_custom_roles",
            enabled: false,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "core_custom_roles",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "core_audit_log",
            enabled: false,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "core_audit_log",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "brand_white_label",
            enabled: false,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "brand_white_label",
            enabled: true,
        },
    ];

    for f in FEATURES {
        sqlx::query(
            r#"
            INSERT INTO plan_features (plan_id, feature_key, value, value_type, enabled)
            SELECT p.id, $2, $3, 'boolean', $4 FROM subscription_plans p WHERE p.slug = $1
            ON CONFLICT (plan_id, feature_key) DO UPDATE SET value = EXCLUDED.value, enabled = EXCLUDED.enabled
            "#
        )
        .bind(f.plan).bind(f.key).bind(if f.enabled { "true" } else { "false" }).bind(f.enabled)
        .execute(pool).await?;
    }
    Ok(())
}

struct LimitSeed {
    plan: &'static str,
    key: &'static str,
    max_value: i32,
    soft_limit: Option<i32>,
}

async fn seed_plan_limits(pool: &PgPool) -> Result<()> {
    const LIMITS: &[LimitSeed] = &[
        // Professional (Units dropped to 150 based on KES 5,000 price point)
        LimitSeed {
            plan: "professional",
            key: "max_units",
            max_value: 150,
            soft_limit: Some(135),
        },
        LimitSeed {
            plan: "professional",
            key: "max_branches",
            max_value: 10,
            soft_limit: Some(8),
        },
        LimitSeed {
            plan: "professional",
            key: "max_storage_mb",
            max_value: 2048,
            soft_limit: Some(1800),
        },
        LimitSeed {
            plan: "professional",
            key: "max_sms_per_month",
            max_value: 1000,
            soft_limit: Some(900),
        },
        LimitSeed {
            plan: "professional",
            key: "max_whatsapp_per_month",
            max_value: 0,
            soft_limit: Some(0),
        },
        // Enterprise (-1 = unlimited)
        LimitSeed {
            plan: "enterprise",
            key: "max_units",
            max_value: -1,
            soft_limit: None,
        },
        LimitSeed {
            plan: "enterprise",
            key: "max_branches",
            max_value: -1,
            soft_limit: None,
        },
        LimitSeed {
            plan: "enterprise",
            key: "max_storage_mb",
            max_value: 51200,
            soft_limit: Some(46080),
        },
        LimitSeed {
            plan: "enterprise",
            key: "max_sms_per_month",
            max_value: 10000,
            soft_limit: Some(9000),
        },
        LimitSeed {
            plan: "enterprise",
            key: "max_whatsapp_per_month",
            max_value: 5000,
            soft_limit: Some(4500),
        },
    ];

    for l in LIMITS {
        sqlx::query(
            r#"
            INSERT INTO plan_limits (plan_id, limit_key, max_value, soft_limit)
            SELECT p.id, $2, $3, $4 FROM subscription_plans p WHERE p.slug = $1
            ON CONFLICT (plan_id, limit_key) DO UPDATE SET max_value = EXCLUDED.max_value, soft_limit = EXCLUDED.soft_limit
            "#
        )
        .bind(l.plan).bind(l.key).bind(l.max_value).bind(l.soft_limit)
        .execute(pool).await?;
    }
    Ok(())
}
