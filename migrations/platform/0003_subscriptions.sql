-- ── ENUMS ─────────────────────────────────────────────────────────────────────

CREATE TYPE subscription_status AS ENUM (
    'active',
    'cancelled',
    'past_due',
    'trialing'
);

-- ── CORE PLAN CATALOGUE ───────────────────────────────────────────────────────

CREATE TABLE subscription_plans (
    id               UUID        NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    slug             TEXT        NOT NULL UNIQUE,
    name             TEXT        NOT NULL,
    description      TEXT,
    price_kes        INTEGER     NOT NULL,
    yearly_price_kes INTEGER,
    interval         TEXT        NOT NULL DEFAULT 'monthly'
                                 CHECK (interval IN ('monthly', 'yearly')),
    is_active        BOOLEAN     NOT NULL DEFAULT true,
    is_public        BOOLEAN     NOT NULL DEFAULT true,
    sort_order       INTEGER     NOT NULL DEFAULT 0,
    trial_days       INTEGER     NOT NULL DEFAULT 14,
    metadata         JSONB,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);


CREATE INDEX idx_subscription_plans_active
    ON subscription_plans (sort_order)
    WHERE is_active = true AND is_public = true;

-- ── PLAN FEATURES ─────────────────────────────────────────────────────────────

CREATE TABLE plan_features (
    id          UUID    NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    plan_id     UUID    NOT NULL REFERENCES subscription_plans (id) ON DELETE CASCADE,
    feature_key TEXT    NOT NULL,
    value_type  TEXT    NOT NULL DEFAULT 'boolean'
                        CHECK (value_type IN ('boolean', 'integer', 'string')),
    value       TEXT    NOT NULL,
    enabled     BOOLEAN NOT NULL DEFAULT true,
    UNIQUE (plan_id, feature_key)
);

CREATE INDEX idx_plan_features_plan ON plan_features (plan_id);

-- ── PLAN LIMITS ───────────────────────────────────────────────────────────────

CREATE TABLE plan_limits (
    id          UUID    NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    plan_id     UUID    NOT NULL REFERENCES subscription_plans (id) ON DELETE CASCADE,
    limit_key   TEXT    NOT NULL,
    max_value   INTEGER NOT NULL,
    soft_limit  INTEGER,
    UNIQUE (plan_id, limit_key)
);

CREATE INDEX idx_plan_limits_plan ON plan_limits (plan_id);

-- ── SUBSCRIPTIONS ─────────────────────────────────────────────────────────────

CREATE TABLE subscriptions (
    id                     UUID                NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    agency_id              UUID                NOT NULL REFERENCES agencies (id) ON DELETE CASCADE,

    -- New relational plan
    plan_id                UUID                REFERENCES subscription_plans (id),

    status                 subscription_status NOT NULL DEFAULT 'trialing',

    started_at             TIMESTAMPTZ         NOT NULL DEFAULT now(),
    ends_at                TIMESTAMPTZ,
    
    plan_tier              TEXT,
    trial_ends_at          TIMESTAMPTZ,
    current_period_start   TIMESTAMPTZ,
    current_period_end     TIMESTAMPTZ,

    cancelled_at           TIMESTAMPTZ,
    cancel_reason          TEXT,
    grace_period_ends_at   TIMESTAMPTZ,

    custom_price_kes       INTEGER,
    last_payment_ref       TEXT,
    last_payment_at        TIMESTAMPTZ,
    payment_failure_count  INTEGER             NOT NULL DEFAULT 0,

    created_at             TIMESTAMPTZ         NOT NULL DEFAULT now(),
    updated_at             TIMESTAMPTZ         NOT NULL DEFAULT now()
);

CREATE INDEX idx_subscriptions_agency
    ON subscriptions (agency_id, status);

-- ── FEATURE OVERRIDES ─────────────────────────────────────────────────────────

CREATE TABLE feature_overrides (
    id          UUID        NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    agency_id   UUID        NOT NULL REFERENCES agencies (id) ON DELETE CASCADE,
    feature_key TEXT        NOT NULL,
    value       TEXT        NOT NULL,
    expires_at  TIMESTAMPTZ,
    reason      TEXT,
    created_by  UUID        REFERENCES users (id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agency_id, feature_key)
);

CREATE INDEX idx_feature_overrides_agency
    ON feature_overrides (agency_id);

CREATE INDEX idx_feature_overrides_expiry
    ON feature_overrides (expires_at)
    WHERE expires_at IS NOT NULL;

-- ── INVOICES ──────────────────────────────────────────────────────────────────

CREATE TABLE subscription_invoices (
    id                     UUID        NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    agency_subscription_id UUID        NOT NULL REFERENCES subscriptions (id),
    agency_id              UUID        NOT NULL REFERENCES agencies (id),

    amount_kes             INTEGER     NOT NULL,

    status                 TEXT        NOT NULL DEFAULT 'draft'
                                       CHECK (status IN (
                                           'draft',
                                           'open',
                                           'paid',
                                           'void',
                                           'uncollectible'
                                       )),

    due_date               TIMESTAMPTZ NOT NULL,
    paid_at                TIMESTAMPTZ,

    mpesa_ref              TEXT,
    mpesa_phone            TEXT,
    receipt_url            TEXT,

    notes                  TEXT,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_invoices_agency ON subscription_invoices (agency_id);
CREATE INDEX idx_invoices_status ON subscription_invoices (status);
CREATE INDEX idx_invoices_due
    ON subscription_invoices (due_date)
    WHERE status = 'open';

-- ── USAGE COUNTERS ────────────────────────────────────────────────────────────

CREATE TABLE agency_usage_counters (
    agency_id   UUID        NOT NULL REFERENCES agencies (id) ON DELETE CASCADE,
    counter_key TEXT        NOT NULL,
    value       BIGINT      NOT NULL DEFAULT 0,
    period      TEXT        NOT NULL DEFAULT 'total',
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (agency_id, counter_key, period)
);

CREATE INDEX idx_usage_agency ON agency_usage_counters (agency_id);

-- ── DEFAULT SUBSCRIPTION TRIGGER ──────────────────────────────────────────────

CREATE OR REPLACE FUNCTION create_default_subscription()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE
    starter_plan_id UUID;
BEGIN
    SELECT id INTO starter_plan_id
    FROM subscription_plans
    WHERE slug = 'starter' AND is_active = true
    LIMIT 1;

    INSERT INTO subscriptions (
        agency_id,
        plan_tier,
        plan_id,
        status,
        trial_ends_at,
        current_period_start,
        current_period_end
    )
    VALUES (
        NEW.id,
        'core',
        starter_plan_id,
        'trialing',
        now() + INTERVAL '14 days',
        now(),
        now() + INTERVAL '1 month'
    );

    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_agency_default_subscription
    AFTER INSERT ON agencies
    FOR EACH ROW
    EXECUTE FUNCTION create_default_subscription();