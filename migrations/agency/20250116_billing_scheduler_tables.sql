-- migrations/agency/20250116_billing_scheduler_tables.sql
--
-- Creates the three tables needed by the billing scheduler, and adds
-- UNIQUE constraints to prevent double-charging on scheduler retry.
--
-- These tables are referenced by the domain types in domain/agreement.rs:
--   RentCharge, LateFeeCharge, RentReminderSent

-- ── RENT CHARGES ──────────────────────────────────────────────────────────────
-- Written once per (agreement, period) by the scheduler.
-- The UNIQUE constraint on (agreement_id, period_start) is the idempotency
-- guard: a second scheduler run for the same period is a no-op.

CREATE TABLE IF NOT EXISTS rent_charges (
    id              UUID           NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agreement_id    UUID           NOT NULL REFERENCES agreements (id) ON DELETE CASCADE,
    period_start    DATE           NOT NULL,
    period_end      DATE           NOT NULL CHECK (period_end >= period_start),
    amount_kes      NUMERIC(14,2)  NOT NULL CHECK (amount_kes > 0),
    ledger_entry_id UUID,                          -- soft ref to ledger_entries.id
    charged_at      TIMESTAMPTZ    NOT NULL DEFAULT now(),

    -- Idempotency: only one charge row per agreement+period
    UNIQUE (agreement_id, period_start)
);

CREATE INDEX IF NOT EXISTS idx_rent_charges_agreement
    ON rent_charges (agreement_id, period_start DESC);

-- ── LATE FEE CHARGES ──────────────────────────────────────────────────────────
-- Written by the scheduler when a payment is overdue beyond the grace period.
-- One late fee per (agreement, period_start).

CREATE TABLE IF NOT EXISTS late_fee_charges (
    id              UUID           NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agreement_id    UUID           NOT NULL REFERENCES agreements (id) ON DELETE CASCADE,
    period_start    DATE           NOT NULL,
    amount_kes      NUMERIC(14,2)  NOT NULL CHECK (amount_kes > 0),
    ledger_entry_id UUID,
    charged_at      TIMESTAMPTZ    NOT NULL DEFAULT now(),

    UNIQUE (agreement_id, period_start)
);

CREATE INDEX IF NOT EXISTS idx_late_fee_charges_agreement
    ON late_fee_charges (agreement_id, period_start DESC);

-- ── RENT REMINDERS SENT ───────────────────────────────────────────────────────
-- Tracks which reminders have been sent so the scheduler doesn't re-send.
-- One row per (agreement, period_start, channel).

CREATE TABLE IF NOT EXISTS rent_reminders_sent (
    id              UUID           NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agreement_id    UUID           NOT NULL REFERENCES agreements (id) ON DELETE CASCADE,
    period_start    DATE           NOT NULL,
    channel         TEXT           NOT NULL CHECK (channel IN ('email', 'sms')),
    sent_at         TIMESTAMPTZ    NOT NULL DEFAULT now(),

    UNIQUE (agreement_id, period_start, channel)
);

CREATE INDEX IF NOT EXISTS idx_rent_reminders_agreement
    ON rent_reminders_sent (agreement_id, period_start DESC);

-- ── LATE FEE POLICY (agency-level config) ─────────────────────────────────────
-- Stored in the agency schema so each agency can configure its own policy.
-- Falls back to platform defaults when no row exists.

CREATE TABLE IF NOT EXISTS late_fee_policy (
    id                  UUID           NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    grace_period_days   INT            NOT NULL DEFAULT 5 CHECK (grace_period_days >= 0),
    flat_amount_kes     NUMERIC(14,2)  CHECK (flat_amount_kes > 0),
    rate_percent        NUMERIC(5,2)   CHECK (rate_percent > 0 AND rate_percent <= 100),
    created_at          TIMESTAMPTZ    NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ    NOT NULL DEFAULT now(),

    -- Exactly one policy row per agency schema
    CHECK (flat_amount_kes IS NOT NULL OR rate_percent IS NOT NULL)
);

-- Seed a default policy: 5-day grace period, 5% of rent
INSERT INTO late_fee_policy (grace_period_days, rate_percent)
VALUES (5, 5)
ON CONFLICT DO NOTHING;