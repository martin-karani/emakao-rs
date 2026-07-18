-- Migration: 0005_payment_reconciliation.sql
-- Extend payment_claims to support reconciliation-driven workflows from
-- tenant app submissions, forwarded SMS messages, and agent-created requests.

ALTER TABLE payment_claims
    ADD COLUMN IF NOT EXISTS unit_id UUID REFERENCES units (id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS submitted_via TEXT NOT NULL DEFAULT 'tenant_app',
    ADD COLUMN IF NOT EXISTS raw_message TEXT,
    ADD COLUMN IF NOT EXISTS period_label TEXT,
    ADD COLUMN IF NOT EXISTS payment_for TEXT,
    ADD COLUMN IF NOT EXISTS allocation JSONB NOT NULL DEFAULT '[]'::jsonb;

CREATE INDEX IF NOT EXISTS idx_payment_claims_unit
    ON payment_claims (unit_id)
    WHERE unit_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_payment_claims_submitted_via
    ON payment_claims (submitted_via, created_at DESC);
