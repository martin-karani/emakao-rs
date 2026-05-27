-- =============================================================================
--
-- Platform database (emakao_platform / public schema) — full baseline.
--
-- Tables (in FK-safe order):
--   1.  agencies
--   2.  users
--   3.  user_agency_roles
--   4.  portal_user_index
--   5.  invite_tokens
--   6.  refresh_tokens
--   7.  password_reset_tokens
--   8.  subscription_plans
--   9.  plan_features
--  10.  plan_limits
--  11.  subscriptions
--  12.  feature_overrides
--  13.  subscription_invoices
--  14.  agency_usage_counters
--  15.  pending_mpesa_subscription_requests
--  16.  agency_custom_domains
--  17.  agency_locale_strings
--  18.  agency_compliance
--  19.  agency_security_policy
--  20.  custom_roles
--  21.  agency_plan_overrides
--  22.  audit_log  (partitioned)
-- =============================================================================

-- ─────────────────────────────────────────────────────────────────────────────
-- HELPERS
-- ─────────────────────────────────────────────────────────────────────────────

CREATE OR REPLACE FUNCTION uuidv7()
RETURNS uuid AS $$
DECLARE
    v_time   timestamp with time zone := clock_timestamp();
    v_unix_t bigint;
    v_rand_a int;
    v_rand_b bigint;
BEGIN
    v_unix_t := (EXTRACT(EPOCH FROM v_time) * 1000)::bigint;
    v_rand_a := floor(random() * 4096)::int;
    v_rand_b := (random() * 4611686018427387904)::bigint;

    RETURN encode(
        set_byte(
            set_byte(
                decode(
                    lpad(to_hex(v_unix_t), 12, '0') ||
                    lpad(to_hex(v_rand_a),  4, '0') ||
                    lpad(to_hex(v_rand_b), 16, '0'),
                    'hex'
                ),
                6,
                (get_byte(decode(lpad(to_hex(v_rand_a), 4, '0'), 'hex'), 0) & 15 | 112)
            ),
            8,
            (get_byte(decode(lpad(to_hex(v_rand_b), 16, '0'), 'hex'), 0) & 63 | 128)
        ),
        'hex'
    )::uuid;
END;
$$ LANGUAGE plpgsql VOLATILE;

-- ─────────────────────────────────────────────────────────────────────────────
-- ENUMS
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TYPE agency_status AS ENUM (
    'active',
    'suspended',
    'deprovisioned'
);

CREATE TYPE user_role AS ENUM (
    'platform_admin',
    'admin',
    'manager',
    'agent',
    'resident',
    'owner',
    'vendor'
);

CREATE TYPE portal_type AS ENUM (
    'staff',
    'resident',
    'owner',
    'vendor'
);

CREATE TYPE contact_type AS ENUM (
    'email',
    'phone'
);

CREATE TYPE subscription_status AS ENUM (
    'trialing',
    'active',
    'past_due',
    'cancelled'
);

CREATE TYPE invoice_status AS ENUM (
    'draft',
    'open',
    'paid',
    'void',
    'uncollectible'
);

CREATE TYPE billing_interval AS ENUM (
    'monthly',
    'yearly'
);

-- ─────────────────────────────────────────────────────────────────────────────
-- 1. agencies
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE agencies (
    id                   UUID          NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    name                 TEXT          NOT NULL CHECK (char_length(trim(name)) > 0),
    slug                 TEXT          NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9\-]+$'),
    schema_name          TEXT          NOT NULL UNIQUE CHECK (schema_name ~ '^[a-z0-9_]+$'),
    country_code         CHAR(2)       NOT NULL DEFAULT 'KE',
    currency_code        CHAR(3)       NOT NULL DEFAULT 'KES',
    status               agency_status NOT NULL DEFAULT 'active',
    fga_store_id         TEXT,
    settings             JSONB         NOT NULL DEFAULT '{}',
    locale               TEXT          NOT NULL DEFAULT 'en-KE',
    subdomain            TEXT          UNIQUE,
    is_vat_registered    BOOLEAN       NOT NULL DEFAULT false,
    is_wht_agent         BOOLEAN       NOT NULL DEFAULT false,
    charge_vat           BOOLEAN       NOT NULL DEFAULT false,
    vat_rate             NUMERIC(5,4)  NOT NULL DEFAULT 0.1600,
    audit_level          TEXT          NOT NULL DEFAULT 'financial'
        CHECK (audit_level IN ('minimal', 'financial', 'full')),
    data_retention_years SMALLINT      NOT NULL DEFAULT 7,
    created_at           TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ   NOT NULL DEFAULT now()
);

CREATE INDEX idx_agencies_slug   ON agencies (slug);
CREATE INDEX idx_agencies_status ON agencies (status);

-- ─────────────────────────────────────────────────────────────────────────────
-- 2. users  (pure identity — no agency coupling)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE users (
    id                   UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    email                TEXT        UNIQUE CHECK (email = lower(trim(email))),
    phone                TEXT        UNIQUE,
    password_hash        TEXT        NOT NULL DEFAULT '',
    is_active            BOOLEAN     NOT NULL DEFAULT true,
    must_change_password BOOLEAN     NOT NULL DEFAULT false,
    last_login_at        TIMESTAMPTZ,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT users_must_have_contact
        CHECK (email IS NOT NULL OR phone IS NOT NULL)
);

CREATE INDEX idx_users_email ON users (email) WHERE email IS NOT NULL;
CREATE INDEX idx_users_phone ON users (phone) WHERE phone IS NOT NULL;

-- ─────────────────────────────────────────────────────────────────────────────
-- 3. user_agency_roles  (one row per person × agency × role)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE user_agency_roles (
    id         UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    user_id    UUID        NOT NULL REFERENCES users    (id) ON DELETE CASCADE,
    agency_id  UUID        NOT NULL REFERENCES agencies (id) ON DELETE CASCADE,
    role       user_role   NOT NULL,
    is_active  BOOLEAN     NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (user_id, agency_id, role)
);

CREATE INDEX idx_uar_user_id   ON user_agency_roles (user_id);
CREATE INDEX idx_uar_agency_id ON user_agency_roles (agency_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 4. portal_user_index  (O(1) login lookup: contact + portal → agency + user)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE portal_user_index (
    contact       TEXT         NOT NULL,
    contact_type  contact_type NOT NULL,
    portal        portal_type  NOT NULL,
    agency_id     UUID         NOT NULL REFERENCES agencies          (id) ON DELETE CASCADE,
    user_id       UUID         NOT NULL REFERENCES users             (id) ON DELETE CASCADE,
    membership_id UUID         NOT NULL REFERENCES user_agency_roles (id) ON DELETE CASCADE,

    PRIMARY KEY (contact, contact_type, portal)
);

CREATE INDEX idx_pui_contact ON portal_user_index (contact, portal);
CREATE INDEX idx_pui_user_id ON portal_user_index (user_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 5. invite_tokens
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE invite_tokens (
    token         TEXT         NOT NULL PRIMARY KEY,
    user_id       UUID         NOT NULL REFERENCES users    (id) ON DELETE CASCADE,
    agency_id     UUID         NOT NULL REFERENCES agencies (id) ON DELETE CASCADE,
    role          user_role    NOT NULL,
    portal        portal_type  NOT NULL,
    contact       TEXT         NOT NULL,
    contact_type  contact_type NOT NULL,
    temp_password TEXT,
    expires_at    TIMESTAMPTZ  NOT NULL DEFAULT now() + INTERVAL '48 hours',
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT now()
);

CREATE INDEX idx_invite_user_id ON invite_tokens (user_id);
CREATE INDEX idx_invite_expires ON invite_tokens (expires_at);

-- ─────────────────────────────────────────────────────────────────────────────
-- 6. refresh_tokens
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE refresh_tokens (
    jti        TEXT        NOT NULL PRIMARY KEY,
    user_id    UUID        NOT NULL REFERENCES users    (id) ON DELETE CASCADE,
    agency_id  UUID        NOT NULL REFERENCES agencies (id) ON DELETE CASCADE,
    role       user_role   NOT NULL,
    portal     portal_type NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_rt_user_id ON refresh_tokens (user_id);
CREATE INDEX idx_rt_expires ON refresh_tokens (expires_at);

-- ─────────────────────────────────────────────────────────────────────────────
-- 7. password_reset_tokens
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE password_reset_tokens (
    id         UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    user_id    UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash TEXT        NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at    TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_prt_token_hash ON password_reset_tokens (token_hash);
CREATE INDEX idx_prt_user_id    ON password_reset_tokens (user_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 8. subscription_plans
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE subscription_plans (
    id               UUID             NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    slug             TEXT             NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9\-]+$'),
    name             TEXT             NOT NULL CHECK (char_length(trim(name)) > 0),
    description      TEXT,
    price_kes        INTEGER          NOT NULL CHECK (price_kes >= 0),
    yearly_price_kes INTEGER          CHECK (yearly_price_kes IS NULL OR yearly_price_kes >= 0),
    interval         billing_interval NOT NULL DEFAULT 'monthly',
    trial_days       INTEGER          NOT NULL DEFAULT 14 CHECK (trial_days >= 0),
    is_active        BOOLEAN          NOT NULL DEFAULT true,
    is_public        BOOLEAN          NOT NULL DEFAULT true,
    sort_order       INTEGER          NOT NULL DEFAULT 0,
    metadata         JSONB            NOT NULL DEFAULT '{}',
    created_at       TIMESTAMPTZ      NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ      NOT NULL DEFAULT now()
);

CREATE INDEX idx_plans_active_public
    ON subscription_plans (sort_order)
    WHERE is_active = true AND is_public = true;

-- ─────────────────────────────────────────────────────────────────────────────
-- 9. plan_features
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE plan_features (
    id          UUID    NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    plan_id     UUID    NOT NULL REFERENCES subscription_plans (id) ON DELETE CASCADE,
    feature_key TEXT    NOT NULL CHECK (char_length(trim(feature_key)) > 0),
    value       TEXT    NOT NULL DEFAULT 'false',
    value_type  TEXT    NOT NULL DEFAULT 'boolean',
    enabled     BOOLEAN NOT NULL DEFAULT false,

    UNIQUE (plan_id, feature_key)
);

CREATE INDEX idx_plan_features_plan_id ON plan_features (plan_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 10. plan_limits
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE plan_limits (
    id         UUID    NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    plan_id    UUID    NOT NULL REFERENCES subscription_plans (id) ON DELETE CASCADE,
    limit_key  TEXT    NOT NULL CHECK (char_length(trim(limit_key)) > 0),
    max_value  INTEGER NOT NULL CHECK (max_value >= -1),   -- -1 = unlimited
    soft_limit INTEGER CHECK (soft_limit IS NULL OR soft_limit >= -1),

    UNIQUE (plan_id, limit_key)
);

CREATE INDEX idx_plan_limits_plan_id ON plan_limits (plan_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 11. subscriptions
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE subscriptions (
    id                    UUID                NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id             UUID                NOT NULL REFERENCES agencies           (id) ON DELETE CASCADE,
    plan_id               UUID                NOT NULL REFERENCES subscription_plans (id),
    status                subscription_status NOT NULL DEFAULT 'trialing',
    plan_tier             TEXT,

    started_at            TIMESTAMPTZ         NOT NULL DEFAULT now(),
    ends_at               TIMESTAMPTZ,
    trial_ends_at         TIMESTAMPTZ,
    current_period_start  TIMESTAMPTZ,
    current_period_end    TIMESTAMPTZ,

    cancelled_at          TIMESTAMPTZ,
    cancel_reason         TEXT,
    grace_period_ends_at  TIMESTAMPTZ,

    custom_price_kes      INTEGER             CHECK (custom_price_kes IS NULL OR custom_price_kes >= 0),
    last_payment_ref      TEXT,
    last_payment_at       TIMESTAMPTZ,
    payment_failure_count INTEGER             NOT NULL DEFAULT 0 CHECK (payment_failure_count >= 0),

    created_at            TIMESTAMPTZ         NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ         NOT NULL DEFAULT now(),

    UNIQUE (agency_id)
);

CREATE INDEX idx_subscriptions_agency_status ON subscriptions (agency_id, status);
CREATE INDEX idx_subscriptions_plan_id       ON subscriptions (plan_id);
CREATE INDEX idx_subscriptions_trial_ends    ON subscriptions (trial_ends_at)
    WHERE status = 'trialing';

-- ─────────────────────────────────────────────────────────────────────────────
-- 12. feature_overrides  (per-agency overrides on top of plan defaults)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE feature_overrides (
    id          UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id   UUID        NOT NULL REFERENCES agencies (id) ON DELETE CASCADE,
    feature_key TEXT        NOT NULL CHECK (char_length(trim(feature_key)) > 0),
    value       TEXT        NOT NULL,
    expires_at  TIMESTAMPTZ,
    reason      TEXT,
    created_by  UUID        REFERENCES users (id) ON DELETE SET NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (agency_id, feature_key)
);

CREATE INDEX idx_feature_overrides_agency_id ON feature_overrides (agency_id);
CREATE INDEX idx_feature_overrides_expiry
    ON feature_overrides (expires_at)
    WHERE expires_at IS NOT NULL;

-- ─────────────────────────────────────────────────────────────────────────────
-- 13. subscription_invoices
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE subscription_invoices (
    id                     UUID           NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_subscription_id UUID           NOT NULL REFERENCES subscriptions (id) ON DELETE CASCADE,
    agency_id              UUID           NOT NULL REFERENCES agencies       (id) ON DELETE CASCADE,
    amount_kes             INTEGER        NOT NULL CHECK (amount_kes > 0),
    status                 invoice_status NOT NULL DEFAULT 'draft',
    due_date               TIMESTAMPTZ    NOT NULL,
    paid_at                TIMESTAMPTZ,
    mpesa_ref              TEXT,
    mpesa_phone            TEXT,
    receipt_url            TEXT,
    notes                  TEXT,
    created_at             TIMESTAMPTZ    NOT NULL DEFAULT now()
);

CREATE INDEX idx_invoices_agency_id ON subscription_invoices (agency_id);
CREATE INDEX idx_invoices_status    ON subscription_invoices (status);
CREATE INDEX idx_invoices_due
    ON subscription_invoices (due_date)
    WHERE status = 'open';

-- ─────────────────────────────────────────────────────────────────────────────
-- 14. agency_usage_counters
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE agency_usage_counters (
    agency_id   UUID        NOT NULL REFERENCES agencies (id) ON DELETE CASCADE,
    counter_key TEXT        NOT NULL CHECK (char_length(trim(counter_key)) > 0),
    value       BIGINT      NOT NULL DEFAULT 0 CHECK (value >= 0),
    period      TEXT        NOT NULL DEFAULT 'total',
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (agency_id, counter_key, period)
);

CREATE INDEX idx_usage_agency_id ON agency_usage_counters (agency_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 15. pending_mpesa_subscription_requests
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE pending_mpesa_subscription_requests (
    id                  UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id           UUID        NOT NULL REFERENCES agencies (id) ON DELETE CASCADE,
    plan_slug           TEXT        NOT NULL,
    amount_kes          INTEGER     NOT NULL,
    checkout_request_id TEXT        NOT NULL UNIQUE,
    merchant_request_id TEXT        NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at          TIMESTAMPTZ NOT NULL DEFAULT now() + INTERVAL '5 minutes'
);

-- ─────────────────────────────────────────────────────────────────────────────
-- 16. agency_custom_domains
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE agency_custom_domains (
    id              UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id       UUID        NOT NULL REFERENCES agencies (id) ON DELETE CASCADE,
    domain          TEXT        NOT NULL UNIQUE,
    verified        BOOLEAN     NOT NULL DEFAULT false,
    cert_issued_at  TIMESTAMPTZ,
    cert_expires_at TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_custom_domains_agency ON agency_custom_domains (agency_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 17. agency_locale_strings  (per-agency i18n overrides)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE agency_locale_strings (
    id        UUID NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id UUID NOT NULL REFERENCES agencies (id) ON DELETE CASCADE,
    locale    TEXT NOT NULL,
    key       TEXT NOT NULL,
    value     TEXT NOT NULL,
    UNIQUE (agency_id, locale, key)
);

CREATE INDEX idx_locale_strings_agency ON agency_locale_strings (agency_id, locale);

-- ─────────────────────────────────────────────────────────────────────────────
-- 18. agency_compliance
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE agency_compliance (
    agency_id              UUID         NOT NULL PRIMARY KEY
                                        REFERENCES agencies (id) ON DELETE CASCADE,
    kra_pin                TEXT,
    vat_number             TEXT,
    wht_rate               NUMERIC(5,4) NOT NULL DEFAULT 0.10,
    county_code            TEXT,
    privacy_policy_url     TEXT,
    consent_text           TEXT,
    dpa_consent_required   BOOLEAN      NOT NULL DEFAULT true,
    aml_reporting_required BOOLEAN      NOT NULL DEFAULT false,
    esign_provider         TEXT,
    data_residency_region  TEXT         NOT NULL DEFAULT 'af-south-1',
    updated_at             TIMESTAMPTZ  NOT NULL DEFAULT now()
);

-- ─────────────────────────────────────────────────────────────────────────────
-- 19. agency_security_policy
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE agency_security_policy (
    agency_id               UUID     NOT NULL PRIMARY KEY
                                     REFERENCES agencies (id) ON DELETE CASCADE,
    mfa_requirement         TEXT     NOT NULL DEFAULT 'optional'
        CHECK (mfa_requirement IN ('none', 'admins_only', 'all_users')),
    session_idle_minutes    SMALLINT NOT NULL DEFAULT 480,
    session_absolute_hours  SMALLINT NOT NULL DEFAULT 24,
    password_min_length     SMALLINT NOT NULL DEFAULT 8,
    password_require_upper  BOOLEAN  NOT NULL DEFAULT false,
    password_require_symbol BOOLEAN  NOT NULL DEFAULT false,
    password_expiry_days    SMALLINT,
    sso_enabled             BOOLEAN  NOT NULL DEFAULT false,
    sso_provider            TEXT,
    sso_metadata_url        TEXT,
    ip_allowlist            JSONB    NOT NULL DEFAULT '[]',
    audit_retention_days    INT      NOT NULL DEFAULT 365,
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ─────────────────────────────────────────────────────────────────────────────
-- 20. custom_roles
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE custom_roles (
    id          UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id   UUID        NOT NULL REFERENCES agencies (id) ON DELETE CASCADE,
    name        TEXT        NOT NULL,
    permissions JSONB       NOT NULL DEFAULT '[]',
    is_system   BOOLEAN     NOT NULL DEFAULT false,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agency_id, name)
);

CREATE INDEX idx_custom_roles_agency ON custom_roles (agency_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 21. agency_plan_overrides  (sales one-offs)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE agency_plan_overrides (
    id          UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id   UUID        NOT NULL REFERENCES agencies (id) ON DELETE CASCADE,
    feature_key TEXT        NOT NULL,
    value       JSONB       NOT NULL,
    reason      TEXT        NOT NULL,
    expires_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agency_id, feature_key)
);

CREATE INDEX idx_plan_overrides_agency ON agency_plan_overrides (agency_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 22. audit_log  (immutable, partitioned monthly)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE audit_log (
    id          UUID        NOT NULL DEFAULT uuidv7(),
    agency_id   UUID        NOT NULL,
    actor_id    UUID,
    actor_role  TEXT,
    action      TEXT        NOT NULL,
    entity_type TEXT        NOT NULL,
    entity_id   UUID        NOT NULL,
    old_data    JSONB,
    new_data    JSONB,
    ip_address  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

-- Seed initial partitions (current month + two ahead).
DO $$
DECLARE
    m   DATE;
    tbl TEXT;
    lo  TEXT;
    hi  TEXT;
BEGIN
    FOR i IN 0..2 LOOP
        m   := date_trunc('month', now())::date + make_interval(months => i);
        tbl := 'audit_log_' || to_char(m, 'YYYY_MM');
        lo  := m::text;
        hi  := (m + interval '1 month')::text;
        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS %I PARTITION OF audit_log
             FOR VALUES FROM (%L) TO (%L)',
            tbl, lo, hi
        );
    END LOOP;
END;
$$;

CREATE INDEX idx_audit_agency_entity ON audit_log (agency_id, entity_type, entity_id);
CREATE INDEX idx_audit_actor         ON audit_log (agency_id, actor_id, created_at DESC);

-- ─────────────────────────────────────────────────────────────────────────────
-- FUNCTIONS & TRIGGERS
-- ─────────────────────────────────────────────────────────────────────────────

CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

DO $$
DECLARE tbl TEXT;
BEGIN
    FOREACH tbl IN ARRAY ARRAY[
        'agencies',
        'users',
        'subscription_plans',
        'subscriptions',
        'agency_compliance',
        'agency_security_policy'
    ]
    LOOP
        EXECUTE format(
            'CREATE TRIGGER trg_%s_updated_at
             BEFORE UPDATE ON %s
             FOR EACH ROW EXECUTE FUNCTION set_updated_at();',
            tbl, tbl
        );
    END LOOP;
END;
$$;

-- Auto-provision a trialing subscription when a new agency is created.
CREATE OR REPLACE FUNCTION create_default_subscription()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE
    starter_plan_id UUID;
BEGIN
    SELECT id INTO starter_plan_id
    FROM   subscription_plans
    WHERE  slug = 'starter' AND is_active = true
    LIMIT  1;

    IF starter_plan_id IS NULL THEN
        RAISE EXCEPTION 'Starter plan not found — seed subscription_plans before creating agencies';
    END IF;

    INSERT INTO subscriptions (
        agency_id,
        plan_id,
        status,
        trial_ends_at,
        current_period_start,
        current_period_end
    ) VALUES (
        NEW.id,
        starter_plan_id,
        'trialing',
        now() + INTERVAL '14 days',
        now(),
        now() + INTERVAL '14 days'
    );

    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_agency_create_subscription
    AFTER INSERT ON agencies
    FOR EACH ROW EXECUTE FUNCTION create_default_subscription();