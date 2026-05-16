-- Add missing columns to subscriptions table
ALTER TABLE subscriptions ADD COLUMN IF NOT EXISTS plan_tier TEXT;
ALTER TABLE subscriptions ADD COLUMN IF NOT EXISTS last_payment_ref TEXT;
ALTER TABLE subscriptions ADD COLUMN IF NOT EXISTS last_payment_at TIMESTAMPTZ;
ALTER TABLE subscriptions ADD COLUMN IF NOT EXISTS payment_failure_count INTEGER DEFAULT 0;

-- Rename column to match the code (if it was called subscription_id)
ALTER TABLE subscription_invoices RENAME COLUMN subscription_id TO agency_subscription_id;

-- Add voided_at to invoices table (if not present)
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS voided_at TIMESTAMPTZ;

-- Create missing pending_mpesa table
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

-- If any work_order table lacks code or work_order_number, add them (they should already be there)
-- These are just safety checks:
ALTER TABLE work_orders ADD COLUMN IF NOT EXISTS code TEXT UNIQUE;
ALTER TABLE work_orders ADD COLUMN IF NOT EXISTS work_order_number INTEGER;