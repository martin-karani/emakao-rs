// crates/migrate/src/seed.rs
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
/// Must be called after platform migrations have run successfully.
pub async fn run_platform_seeds(pool: &PgPool) -> Result<()> {
    seed_subscription_plans(pool).await?;
    seed_plan_features(pool).await?;
    seed_plan_limits(pool).await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Subscription plans
// ─────────────────────────────────────────────────────────────────────────────

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
            slug: "starter",
            name: "Starter",
            description: "Perfect for independent landlords and small agencies.",
            price_kes: 2_500,
            yearly_price_kes: 25_000,
            trial_days: 14,
            sort_order: 1,
        },
        PlanSeed {
            slug: "growth",
            name: "Growth",
            description: "For growing agencies that need advanced features.",
            price_kes: 8_000,
            yearly_price_kes: 80_000,
            trial_days: 14,
            sort_order: 2,
        },
        PlanSeed {
            slug: "enterprise",
            name: "Enterprise",
            description: "Unlimited scale for large property management firms.",
            price_kes: 25_000,
            yearly_price_kes: 250_000,
            trial_days: 14,
            sort_order: 3,
        },
    ];

    for p in PLANS {
        sqlx::query(
            r#"
            INSERT INTO subscription_plans
                (slug, name, description, price_kes, yearly_price_kes,
                 billing_interval, trial_days, is_active, is_public, sort_order)
            VALUES ($1, $2, $3, $4, $5, 'monthly', $6, true, true, $7)
            ON CONFLICT (slug) DO UPDATE SET
                name             = EXCLUDED.name,
                description      = EXCLUDED.description,
                price_kes        = EXCLUDED.price_kes,
                yearly_price_kes = EXCLUDED.yearly_price_kes,
                trial_days       = EXCLUDED.trial_days,
                sort_order       = EXCLUDED.sort_order,
                updated_at       = now()
            "#,
        )
        .bind(p.slug)
        .bind(p.name)
        .bind(p.description)
        .bind(p.price_kes)
        .bind(p.yearly_price_kes)
        .bind(p.trial_days)
        .bind(p.sort_order)
        .execute(pool)
        .await
        .with_context(|| format!("failed to seed plan '{}'", p.slug))?;
    }

    println!("  ✓ subscription_plans seeded ({} plans)", PLANS.len());
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Plan features
// ─────────────────────────────────────────────────────────────────────────────

struct FeatureSeed {
    plan: &'static str,
    key: &'static str,
    value: &'static str,
    enabled: bool,
}

// One flat list — easier to scan / diff than three separate blocks.
// Order: starter → growth → enterprise, grouped by feature category.
const FEATURES: &[FeatureSeed] = &[
    // ── core ─────────────────────────────────────────────────────────────────
    FeatureSeed {
        plan: "starter",
        key: "core_multi_branch",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "core_multi_branch",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "core_multi_branch",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "core_roles_rbac",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "core_roles_rbac",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "core_roles_rbac",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "core_audit_log",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "core_audit_log",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "core_audit_log",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "core_two_factor",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "core_two_factor",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "core_two_factor",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "core_api_access",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "core_api_access",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "core_api_access",
        value: "true",
        enabled: true,
    },
    // ── payment ───────────────────────────────────────────────────────────────
    FeatureSeed {
        plan: "starter",
        key: "payment_mpesa_stk",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "growth",
        key: "payment_mpesa_stk",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "payment_mpesa_stk",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "payment_mpesa_paybill",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "growth",
        key: "payment_mpesa_paybill",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "payment_mpesa_paybill",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "payment_mpesa_till",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "payment_mpesa_till",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "payment_mpesa_till",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "payment_bank_transfer",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "payment_bank_transfer",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "payment_bank_transfer",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "payment_auto_receipts",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "growth",
        key: "payment_auto_receipts",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "payment_auto_receipts",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "payment_reminders",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "growth",
        key: "payment_reminders",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "payment_reminders",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "payment_late_fees",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "payment_late_fees",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "payment_late_fees",
        value: "true",
        enabled: true,
    },
    // ── comm ─────────────────────────────────────────────────────────────────
    FeatureSeed {
        plan: "starter",
        key: "comm_sms_basic",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "growth",
        key: "comm_sms_basic",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "comm_sms_basic",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "comm_sms_branded",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "comm_sms_branded",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "comm_sms_branded",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "comm_whatsapp",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "comm_whatsapp",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "comm_whatsapp",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "comm_email_campaigns",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "comm_email_campaigns",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "comm_email_campaigns",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "comm_tenant_portal",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "growth",
        key: "comm_tenant_portal",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "comm_tenant_portal",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "comm_tenant_app",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "comm_tenant_app",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "comm_tenant_app",
        value: "true",
        enabled: true,
    },
    // ── accounting ────────────────────────────────────────────────────────────
    FeatureSeed {
        plan: "starter",
        key: "acct_double_entry",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "acct_double_entry",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "acct_double_entry",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "acct_vat_reports",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "acct_vat_reports",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "acct_vat_reports",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "acct_management_fee",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "acct_management_fee",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "acct_management_fee",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "acct_expense_tracking",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "growth",
        key: "acct_expense_tracking",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "acct_expense_tracking",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "acct_bank_reconciliation",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "acct_bank_reconciliation",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "acct_bank_reconciliation",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "acct_quickbooks_sync",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "acct_quickbooks_sync",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "acct_quickbooks_sync",
        value: "true",
        enabled: true,
    },
    // ── reports ───────────────────────────────────────────────────────────────
    FeatureSeed {
        plan: "starter",
        key: "report_occupancy",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "growth",
        key: "report_occupancy",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "report_occupancy",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "report_arrears_aging",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "growth",
        key: "report_arrears_aging",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "report_arrears_aging",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "report_cashflow",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "report_cashflow",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "report_cashflow",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "report_custom_builder",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "report_custom_builder",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "report_custom_builder",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "report_scheduled_export",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "report_scheduled_export",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "report_scheduled_export",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "report_portfolio_summary",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "report_portfolio_summary",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "report_portfolio_summary",
        value: "true",
        enabled: true,
    },
    // ── maintenance ───────────────────────────────────────────────────────────
    FeatureSeed {
        plan: "starter",
        key: "maint_work_orders",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "growth",
        key: "maint_work_orders",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "maint_work_orders",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "maint_vendor_portal",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "maint_vendor_portal",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "maint_vendor_portal",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "maint_preventive_sched",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "maint_preventive_sched",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "maint_preventive_sched",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "maint_asset_register",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "maint_asset_register",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "maint_asset_register",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "maint_amenity_bookings",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "maint_amenity_bookings",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "maint_amenity_bookings",
        value: "true",
        enabled: true,
    },
    // ── owner portal ──────────────────────────────────────────────────────────
    FeatureSeed {
        plan: "starter",
        key: "owner_portal",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "owner_portal",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "owner_portal",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "owner_statements",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "owner_statements",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "owner_statements",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "owner_disbursements",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "owner_disbursements",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "owner_disbursements",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "owner_document_sharing",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "owner_document_sharing",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "owner_document_sharing",
        value: "true",
        enabled: true,
    },
    // ── lease ─────────────────────────────────────────────────────────────────
    FeatureSeed {
        plan: "starter",
        key: "lease_digital_signing",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "lease_digital_signing",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "lease_digital_signing",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "lease_lease_templates",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "lease_lease_templates",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "lease_lease_templates",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "lease_escalation_rules",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "lease_escalation_rules",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "lease_escalation_rules",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "lease_multi_currency",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "lease_multi_currency",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "lease_multi_currency",
        value: "true",
        enabled: true,
    },
    // ── branding ──────────────────────────────────────────────────────────────
    FeatureSeed {
        plan: "starter",
        key: "brand_white_label",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "brand_white_label",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "brand_white_label",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "brand_custom_domain",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "brand_custom_domain",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "brand_custom_domain",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "brand_custom_email_domain",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "brand_custom_email_domain",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "brand_custom_email_domain",
        value: "true",
        enabled: true,
    },
    // ── marketing ─────────────────────────────────────────────────────────────
    FeatureSeed {
        plan: "starter",
        key: "mktg_listings_page",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "growth",
        key: "mktg_listings_page",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "mktg_listings_page",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "mktg_website_embed",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "mktg_website_embed",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "mktg_website_embed",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "mktg_lead_capture",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "mktg_lead_capture",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "mktg_lead_capture",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "mktg_virtual_tours",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "mktg_virtual_tours",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "mktg_virtual_tours",
        value: "true",
        enabled: true,
    },
    // ── integrations ──────────────────────────────────────────────────────────
    FeatureSeed {
        plan: "starter",
        key: "integ_utility_billing",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "integ_utility_billing",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "integ_utility_billing",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "integ_credit_check",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "integ_credit_check",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "integ_credit_check",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "integ_county_permits",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "integ_county_permits",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "integ_county_permits",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "integ_webhooks",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "integ_webhooks",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "integ_webhooks",
        value: "true",
        enabled: true,
    },
    FeatureSeed {
        plan: "starter",
        key: "integ_zapier",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "growth",
        key: "integ_zapier",
        value: "false",
        enabled: false,
    },
    FeatureSeed {
        plan: "enterprise",
        key: "integ_zapier",
        value: "true",
        enabled: true,
    },
];

async fn seed_plan_features(pool: &PgPool) -> Result<()> {
    for f in FEATURES {
        sqlx::query(
            r#"
            INSERT INTO plan_features (plan_id, feature_key, value, enabled)
            SELECT p.id, $2, $3, $4
            FROM   subscription_plans p
            WHERE  p.slug = $1
            ON CONFLICT (plan_id, feature_key) DO UPDATE SET
                value   = EXCLUDED.value,
                enabled = EXCLUDED.enabled
            "#,
        )
        .bind(f.plan)
        .bind(f.key)
        .bind(f.value)
        .bind(f.enabled)
        .execute(pool)
        .await
        .with_context(|| format!("failed to seed feature '{}/{}' ", f.plan, f.key))?;
    }

    println!("  ✓ plan_features seeded ({} rows)", FEATURES.len());
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Plan limits
// ─────────────────────────────────────────────────────────────────────────────

struct LimitSeed {
    plan: &'static str,
    key: &'static str,
    max_value: i32,
    soft_limit: Option<i32>,
}

// -1 in max_value means unlimited.
const LIMITS: &[LimitSeed] = &[
    // ── starter ──────────────────────────────────────────────────────────────
    LimitSeed {
        plan: "starter",
        key: "max_users",
        max_value: 5,
        soft_limit: Some(4),
    },
    LimitSeed {
        plan: "starter",
        key: "max_properties",
        max_value: 10,
        soft_limit: Some(9),
    },
    LimitSeed {
        plan: "starter",
        key: "max_units",
        max_value: 50,
        soft_limit: Some(45),
    },
    LimitSeed {
        plan: "starter",
        key: "max_owners",
        max_value: 10,
        soft_limit: Some(9),
    },
    LimitSeed {
        plan: "starter",
        key: "max_branches",
        max_value: 1,
        soft_limit: Some(1),
    },
    LimitSeed {
        plan: "starter",
        key: "max_storage_mb",
        max_value: 2048,
        soft_limit: Some(1843),
    },
    LimitSeed {
        plan: "starter",
        key: "max_sms_per_month",
        max_value: 200,
        soft_limit: Some(180),
    },
    LimitSeed {
        plan: "starter",
        key: "max_whatsapp_per_month",
        max_value: 0,
        soft_limit: Some(0),
    },
    LimitSeed {
        plan: "starter",
        key: "max_api_calls_per_day",
        max_value: 0,
        soft_limit: Some(0),
    },
    LimitSeed {
        plan: "starter",
        key: "max_custom_reports",
        max_value: 0,
        soft_limit: Some(0),
    },
    LimitSeed {
        plan: "starter",
        key: "max_document_templates",
        max_value: 3,
        soft_limit: Some(3),
    },
    LimitSeed {
        plan: "starter",
        key: "max_applicants",
        max_value: 2000,
        soft_limit: Some(1800),
    },
    LimitSeed {
        plan: "starter",
        key: "max_vendors",
        max_value: 200,
        soft_limit: Some(180),
    },
    // ── growth ───────────────────────────────────────────────────────────────
    LimitSeed {
        plan: "growth",
        key: "max_users",
        max_value: 25,
        soft_limit: Some(22),
    },
    LimitSeed {
        plan: "growth",
        key: "max_properties",
        max_value: 60,
        soft_limit: Some(54),
    },
    LimitSeed {
        plan: "growth",
        key: "max_units",
        max_value: 300,
        soft_limit: Some(270),
    },
    LimitSeed {
        plan: "growth",
        key: "max_owners",
        max_value: 100,
        soft_limit: Some(90),
    },
    LimitSeed {
        plan: "growth",
        key: "max_branches",
        max_value: 3,
        soft_limit: Some(3),
    },
    LimitSeed {
        plan: "growth",
        key: "max_storage_mb",
        max_value: 10240,
        soft_limit: Some(9216),
    },
    LimitSeed {
        plan: "growth",
        key: "max_sms_per_month",
        max_value: 1000,
        soft_limit: Some(900),
    },
    LimitSeed {
        plan: "growth",
        key: "max_whatsapp_per_month",
        max_value: 500,
        soft_limit: Some(450),
    },
    LimitSeed {
        plan: "growth",
        key: "max_api_calls_per_day",
        max_value: 5000,
        soft_limit: Some(4500),
    },
    LimitSeed {
        plan: "growth",
        key: "max_custom_reports",
        max_value: 20,
        soft_limit: Some(18),
    },
    LimitSeed {
        plan: "growth",
        key: "max_document_templates",
        max_value: 20,
        soft_limit: Some(18),
    },
    LimitSeed {
        plan: "growth",
        key: "max_applicants",
        max_value: 2000,
        soft_limit: Some(1800),
    },
    LimitSeed {
        plan: "growth",
        key: "max_vendors",
        max_value: 200,
        soft_limit: Some(180),
    },
    // ── enterprise — -1 = unlimited ───────────────────────────────────────────
    LimitSeed {
        plan: "enterprise",
        key: "max_users",
        max_value: -1,
        soft_limit: None,
    },
    LimitSeed {
        plan: "enterprise",
        key: "max_properties",
        max_value: -1,
        soft_limit: None,
    },
    LimitSeed {
        plan: "enterprise",
        key: "max_units",
        max_value: -1,
        soft_limit: None,
    },
    LimitSeed {
        plan: "enterprise",
        key: "max_owners",
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
        max_value: 102400,
        soft_limit: Some(92160),
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
    LimitSeed {
        plan: "enterprise",
        key: "max_api_calls_per_day",
        max_value: -1,
        soft_limit: None,
    },
    LimitSeed {
        plan: "enterprise",
        key: "max_custom_reports",
        max_value: -1,
        soft_limit: None,
    },
    LimitSeed {
        plan: "enterprise",
        key: "max_document_templates",
        max_value: -1,
        soft_limit: None,
    },
    LimitSeed {
        plan: "enterprise",
        key: "max_applicants",
        max_value: -1,
        soft_limit: None,
    },
    LimitSeed {
        plan: "enterprise",
        key: "max_vendors",
        max_value: -1,
        soft_limit: None,
    },
];

async fn seed_plan_limits(pool: &PgPool) -> Result<()> {
    for l in LIMITS {
        sqlx::query(
            r#"
            INSERT INTO plan_limits (plan_id, limit_key, max_value, soft_limit)
            SELECT p.id, $2, $3, $4
            FROM   subscription_plans p
            WHERE  p.slug = $1
            ON CONFLICT (plan_id, limit_key) DO UPDATE SET
                max_value  = EXCLUDED.max_value,
                soft_limit = EXCLUDED.soft_limit
            "#,
        )
        .bind(l.plan)
        .bind(l.key)
        .bind(l.max_value)
        .bind(l.soft_limit)
        .execute(pool)
        .await
        .with_context(|| format!("failed to seed limit '{}/{}' ", l.plan, l.key))?;
    }

    println!("  ✓ plan_limits seeded ({} rows)", LIMITS.len());
    Ok(())
}
