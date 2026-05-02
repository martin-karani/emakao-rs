
-- ── 0005_seed_plans.sql ───────────────────────────────────────────────────────
-- Seeds starter / growth / enterprise plans.
-- Safe to re-run (ON CONFLICT DO UPDATE).

DO $$
DECLARE
    starter_id    UUID;
    growth_id     UUID;
    enterprise_id UUID;
BEGIN

-- ── Plans ─────────────────────────────────────────────────────────────────────

INSERT INTO subscription_plans
    (slug, name, description, price_kes, yearly_price_kes, trial_days, sort_order, metadata)
VALUES
    ('starter', 'Starter',
     'Perfect for independent agents and small portfolios',
     4999, 49990, 14, 1,
     '{"badge":null,"highlight":"Get started in minutes","targetAudience":"Independent agents — up to 50 units"}'::jsonb),

    ('growth', 'Growth',
     'For established agencies scaling their portfolio',
     14999, 149990, 14, 2,
     '{"badge":"Most popular","highlight":"Everything you need to run a professional agency","targetAudience":"Growing agencies — 50–300 units"}'::jsonb),

    ('enterprise', 'Enterprise',
     'For large agencies managing hundreds of properties across Kenya',
     49999, 499990, 30, 3,
     '{"badge":"Best value for large agencies","highlight":"Unlimited scale, white-label, dedicated support","targetAudience":"Large agencies, developers, REITs — 300+ units"}'::jsonb)
ON CONFLICT (slug) DO UPDATE
    SET price_kes        = EXCLUDED.price_kes,
        yearly_price_kes = EXCLUDED.yearly_price_kes,
        description      = EXCLUDED.description,
        metadata         = EXCLUDED.metadata,
        updated_at       = now();

SELECT id INTO starter_id    FROM subscription_plans WHERE slug = 'starter';
SELECT id INTO growth_id     FROM subscription_plans WHERE slug = 'growth';
SELECT id INTO enterprise_id FROM subscription_plans WHERE slug = 'enterprise';

-- ── Starter features ──────────────────────────────────────────────────────────

INSERT INTO plan_features (plan_id, feature_key, value, enabled) VALUES
    -- Core
    (starter_id, 'core_multi_branch',         'false', false),
    (starter_id, 'core_roles_rbac',           'false', false),
    (starter_id, 'core_audit_log',            'false', false),
    (starter_id, 'core_two_factor',           'false', false),
    (starter_id, 'core_api_access',           'false', false),
    -- Payments
    (starter_id, 'payment_mpesa_stk',         'true',  true),
    (starter_id, 'payment_mpesa_paybill',     'false', false),
    (starter_id, 'payment_mpesa_till',        'false', false),
    (starter_id, 'payment_bank_transfer',     'false', false),
    (starter_id, 'payment_auto_receipts',     'true',  true),
    (starter_id, 'payment_reminders',         'true',  true),
    (starter_id, 'payment_late_fees',         'false', false),
    -- Communications
    (starter_id, 'comm_sms_basic',            'true',  true),
    (starter_id, 'comm_sms_branded',          'false', false),
    (starter_id, 'comm_whatsapp',             'false', false),
    (starter_id, 'comm_email_campaigns',      'false', false),
    (starter_id, 'comm_tenant_portal',        'true',  true),
    (starter_id, 'comm_tenant_app',           'false', false),
    -- Accounting
    (starter_id, 'acct_double_entry',         'false', false),
    (starter_id, 'acct_vat_reports',          'false', false),
    (starter_id, 'acct_management_fee',       'true',  true),
    (starter_id, 'acct_expense_tracking',     'false', false),
    (starter_id, 'acct_bank_reconciliation',  'false', false),
    (starter_id, 'acct_quickbooks_sync',      'false', false),
    -- Reporting
    (starter_id, 'report_occupancy',          'true',  true),
    (starter_id, 'report_arrears_aging',      'true',  true),
    (starter_id, 'report_cashflow',           'false', false),
    (starter_id, 'report_custom_builder',     'false', false),
    (starter_id, 'report_scheduled_export',   'false', false),
    (starter_id, 'report_portfolio_summary',  'false', false),
    -- Maintenance
    (starter_id, 'maint_work_orders',         'true',  true),
    (starter_id, 'maint_vendor_portal',       'false', false),
    (starter_id, 'maint_preventive_sched',    'false', false),
    (starter_id, 'maint_asset_register',      'false', false),
    (starter_id, 'maint_amenity_bookings',    'false', false),
    -- Owner portal
    (starter_id, 'owner_portal',              'false', false),
    (starter_id, 'owner_statements',          'false', false),
    (starter_id, 'owner_disbursements',       'false', false),
    (starter_id, 'owner_document_sharing',    'false', false),
    -- Lease
    (starter_id, 'lease_digital_signing',     'false', false),
    (starter_id, 'lease_lease_templates',     'false', false),
    (starter_id, 'lease_escalation_rules',    'false', false),
    (starter_id, 'lease_multi_currency',      'false', false),
    -- Branding
    (starter_id, 'brand_white_label',         'false', false),
    (starter_id, 'brand_custom_domain',       'false', false),
    (starter_id, 'brand_custom_email_domain', 'false', false),
    -- Marketing
    (starter_id, 'mktg_listings_page',        'true',  true),
    (starter_id, 'mktg_website_embed',        'false', false),
    (starter_id, 'mktg_lead_capture',         'false', false),
    (starter_id, 'mktg_virtual_tours',        'false', false),
    -- Integrations
    (starter_id, 'integ_utility_billing',     'false', false),
    (starter_id, 'integ_credit_check',        'false', false),
    (starter_id, 'integ_county_permits',      'false', false),
    (starter_id, 'integ_webhooks',            'false', false),
    (starter_id, 'integ_zapier',              'false', false)
ON CONFLICT (plan_id, feature_key) DO UPDATE
    SET value = EXCLUDED.value, enabled = EXCLUDED.enabled;

-- ── Starter limits ────────────────────────────────────────────────────────────

INSERT INTO plan_limits (plan_id, limit_key, max_value, soft_limit) VALUES
    (starter_id, 'max_users',               5,     4),
    (starter_id, 'max_properties',          10,    9),
    (starter_id, 'max_units',               50,    45),
    (starter_id, 'max_owners',              10,    9),
    (starter_id, 'max_branches',            1,     1),
    (starter_id, 'max_storage_mb',          2048,  1843),
    (starter_id, 'max_sms_per_month',       200,   180),
    (starter_id, 'max_whatsapp_per_month',  0,     0),
    (starter_id, 'max_api_calls_per_day',   0,     0),
    (starter_id, 'max_custom_reports',      0,     0),
    (starter_id, 'max_document_templates',  3,     3),
    (starter_id, 'max_applicants',          2000,  1800),
    (starter_id, 'max_vendors',             200,   180)
ON CONFLICT (plan_id, limit_key) DO UPDATE
    SET max_value = EXCLUDED.max_value, soft_limit = EXCLUDED.soft_limit;

-- ── Growth features ───────────────────────────────────────────────────────────

INSERT INTO plan_features (plan_id, feature_key, value, enabled) VALUES
    (growth_id, 'core_multi_branch',         'false', false),
    (growth_id, 'core_roles_rbac',           'true',  true),
    (growth_id, 'core_audit_log',            'true',  true),
    (growth_id, 'core_two_factor',           'true',  true),
    (growth_id, 'core_api_access',           'true',  true),
    (growth_id, 'payment_mpesa_stk',         'true',  true),
    (growth_id, 'payment_mpesa_paybill',     'true',  true),
    (growth_id, 'payment_mpesa_till',        'false', false),
    (growth_id, 'payment_bank_transfer',     'false', false),
    (growth_id, 'payment_auto_receipts',     'true',  true),
    (growth_id, 'payment_reminders',         'true',  true),
    (growth_id, 'payment_late_fees',         'true',  true),
    (growth_id, 'comm_sms_basic',            'true',  true),
    (growth_id, 'comm_sms_branded',          'true',  true),
    (growth_id, 'comm_whatsapp',             'true',  true),
    (growth_id, 'comm_email_campaigns',      'false', false),
    (growth_id, 'comm_tenant_portal',        'true',  true),
    (growth_id, 'comm_tenant_app',           'true',  true),
    (growth_id, 'acct_double_entry',         'true',  true),
    (growth_id, 'acct_vat_reports',          'true',  true),
    (growth_id, 'acct_management_fee',       'true',  true),
    (growth_id, 'acct_expense_tracking',     'true',  true),
    (growth_id, 'acct_bank_reconciliation',  'true',  true),
    (growth_id, 'acct_quickbooks_sync',      'false', false),
    (growth_id, 'report_occupancy',          'true',  true),
    (growth_id, 'report_arrears_aging',      'true',  true),
    (growth_id, 'report_cashflow',           'true',  true),
    (growth_id, 'report_custom_builder',     'true',  true),
    (growth_id, 'report_scheduled_export',   'true',  true),
    (growth_id, 'report_portfolio_summary',  'false', false),
    (growth_id, 'maint_work_orders',         'true',  true),
    (growth_id, 'maint_vendor_portal',       'true',  true),
    (growth_id, 'maint_preventive_sched',    'true',  true),
    (growth_id, 'maint_asset_register',      'true',  true),
    (growth_id, 'maint_amenity_bookings',    'true',  true),
    (growth_id, 'owner_portal',              'true',  true),
    (growth_id, 'owner_statements',          'true',  true),
    (growth_id, 'owner_disbursements',       'true',  true),
    (growth_id, 'owner_document_sharing',    'false', false),
    (growth_id, 'lease_digital_signing',     'true',  true),
    (growth_id, 'lease_lease_templates',     'true',  true),
    (growth_id, 'lease_escalation_rules',    'true',  true),
    (growth_id, 'lease_multi_currency',      'false', false),
    (growth_id, 'brand_white_label',         'false', false),
    (growth_id, 'brand_custom_domain',       'true',  true),
    (growth_id, 'brand_custom_email_domain', 'false', false),
    (growth_id, 'mktg_listings_page',        'true',  true),
    (growth_id, 'mktg_website_embed',        'true',  true),
    (growth_id, 'mktg_lead_capture',         'false', false),
    (growth_id, 'mktg_virtual_tours',        'false', false),
    (growth_id, 'integ_utility_billing',     'true',  true),
    (growth_id, 'integ_credit_check',        'true',  true),
    (growth_id, 'integ_county_permits',      'false', false),
    (growth_id, 'integ_webhooks',            'true',  true),
    (growth_id, 'integ_zapier',              'false', false)
ON CONFLICT (plan_id, feature_key) DO UPDATE
    SET value = EXCLUDED.value, enabled = EXCLUDED.enabled;

-- ── Growth limits ─────────────────────────────────────────────────────────────

INSERT INTO plan_limits (plan_id, limit_key, max_value, soft_limit) VALUES
    (growth_id, 'max_users',               25,    22),
    (growth_id, 'max_properties',          60,    54),
    (growth_id, 'max_units',               300,   270),
    (growth_id, 'max_owners',              100,   90),
    (growth_id, 'max_branches',            3,     3),
    (growth_id, 'max_storage_mb',          10240, 9216),
    (growth_id, 'max_sms_per_month',       1000,  900),
    (growth_id, 'max_whatsapp_per_month',  500,   450),
    (growth_id, 'max_api_calls_per_day',   5000,  4500),
    (growth_id, 'max_custom_reports',      20,    18),
    (growth_id, 'max_document_templates',  20,    18),
    (growth_id, 'max_applicants',          2000,  1800),
    (growth_id, 'max_vendors',             200,   180)
ON CONFLICT (plan_id, limit_key) DO UPDATE
    SET max_value = EXCLUDED.max_value, soft_limit = EXCLUDED.soft_limit;

-- ── Enterprise features (everything = true) ───────────────────────────────────

INSERT INTO plan_features (plan_id, feature_key, value, enabled)
SELECT enterprise_id, feature_key, 'true', true
FROM (VALUES
    ('core_multi_branch'),('core_roles_rbac'),('core_audit_log'),
    ('core_two_factor'),('core_api_access'),
    ('payment_mpesa_stk'),('payment_mpesa_paybill'),('payment_mpesa_till'),
    ('payment_bank_transfer'),('payment_auto_receipts'),
    ('payment_reminders'),('payment_late_fees'),
    ('comm_sms_basic'),('comm_sms_branded'),('comm_whatsapp'),
    ('comm_email_campaigns'),('comm_tenant_portal'),('comm_tenant_app'),
    ('acct_double_entry'),('acct_vat_reports'),('acct_management_fee'),
    ('acct_expense_tracking'),('acct_bank_reconciliation'),('acct_quickbooks_sync'),
    ('report_occupancy'),('report_arrears_aging'),('report_cashflow'),
    ('report_custom_builder'),('report_scheduled_export'),('report_portfolio_summary'),
    ('maint_work_orders'),('maint_vendor_portal'),('maint_preventive_sched'),
    ('maint_asset_register'),('maint_amenity_bookings'),
    ('owner_portal'),('owner_statements'),('owner_disbursements'),('owner_document_sharing'),
    ('lease_digital_signing'),('lease_lease_templates'),
    ('lease_escalation_rules'),('lease_multi_currency'),
    ('brand_white_label'),('brand_custom_domain'),('brand_custom_email_domain'),
    ('mktg_listings_page'),('mktg_website_embed'),('mktg_lead_capture'),('mktg_virtual_tours'),
    ('integ_utility_billing'),('integ_credit_check'),('integ_county_permits'),
    ('integ_webhooks'),('integ_zapier')
) AS t(feature_key)
ON CONFLICT (plan_id, feature_key) DO UPDATE
    SET value = 'true', enabled = true;

-- ── Enterprise limits (-1 = unlimited) ───────────────────────────────────────

INSERT INTO plan_limits (plan_id, limit_key, max_value, soft_limit) VALUES
    (enterprise_id, 'max_users',               -1,     NULL),
    (enterprise_id, 'max_properties',           -1,     NULL),
    (enterprise_id, 'max_units',                -1,     NULL),
    (enterprise_id, 'max_owners',               -1,     NULL),
    (enterprise_id, 'max_branches',             -1,     NULL),
    (enterprise_id, 'max_storage_mb',           102400, 92160),
    (enterprise_id, 'max_sms_per_month',        10000,  9000),
    (enterprise_id, 'max_whatsapp_per_month',   5000,   4500),
    (enterprise_id, 'max_api_calls_per_day',    -1,     NULL),
    (enterprise_id, 'max_custom_reports',       -1,     NULL),
    (enterprise_id, 'max_document_templates',   -1,     NULL),
    (enterprise_id, 'max_applicants',           -1,     NULL),
    (enterprise_id, 'max_vendors',              -1,     NULL)
ON CONFLICT (plan_id, limit_key) DO UPDATE
    SET max_value = EXCLUDED.max_value, soft_limit = EXCLUDED.soft_limit;

-- ── Back-fill plan_id on existing subscriptions ───────────────────────────────

UPDATE subscriptions s
SET    plan_id = p.id
FROM   subscription_plans p
WHERE  s.plan_id IS NULL
  AND  (
           (s.plan_tier = 'core'  AND p.slug = 'starter')
        OR (s.plan_tier = 'plus'  AND p.slug = 'growth')
        OR (s.plan_tier = 'max'   AND p.slug = 'enterprise')
       );

-- Back-fill trial_ends_at for trialing rows that don't have it yet
UPDATE subscriptions
SET    trial_ends_at = started_at + INTERVAL '14 days'
WHERE  status        = 'trialing'
  AND  trial_ends_at IS NULL;

END $$;