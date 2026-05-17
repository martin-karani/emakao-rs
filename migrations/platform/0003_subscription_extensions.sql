-- migrations/platform/0003_subscription_extensions.sql

-- 1. subscription_plans: billing_interval -> interval
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subscription_plans' AND column_name = 'billing_interval') THEN
        ALTER TABLE subscription_plans RENAME COLUMN billing_interval TO interval;
    END IF;
END $$;

-- 2. plan_features: add value_type
ALTER TABLE plan_features ADD COLUMN IF NOT EXISTS value_type TEXT NOT NULL DEFAULT 'boolean';

-- 3. subscriptions: add plan_tier
ALTER TABLE subscriptions ADD COLUMN IF NOT EXISTS plan_tier TEXT;

-- 4. subscription_invoices: subscription_id -> agency_subscription_id
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subscription_invoices' AND column_name = 'subscription_id') THEN
        ALTER TABLE subscription_invoices RENAME COLUMN subscription_id TO agency_subscription_id;
    END IF;
END $$;

-- 5. pending_mpesa_subscription_requests
CREATE TABLE IF NOT EXISTS pending_mpesa_subscription_requests (
    id                  UUID PRIMARY KEY DEFAULT uuidv7(),
    agency_id           UUID NOT NULL REFERENCES agencies(id) ON DELETE CASCADE,
    plan_slug           TEXT NOT NULL,
    amount_kes          INTEGER NOT NULL,
    checkout_request_id TEXT NOT NULL UNIQUE,
    merchant_request_id TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at          TIMESTAMPTZ NOT NULL DEFAULT now() + INTERVAL '5 minutes'
);
