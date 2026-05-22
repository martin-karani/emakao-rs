-- =============================================================================
-- migrations/platform/0002_customisation.sql
--
-- Multi-tenant customisation layer — platform side.
--
-- Adds to `agencies`:
--   settings JSONB, locale, currency_code (already exists, kept idempotent),
--   subdomain, is_vat_registered, is_wht_agent, charge_vat, vat_rate,
--   audit_level, data_retention_years
--
-- New tables (in FK-safe order):
--   1.  agency_custom_domains
--   2.  agency_locale_strings
--   3.  agency_compliance
--   4.  agency_security_policy
--   5.  custom_roles
--   6.  agency_plan_overrides
--   7.  audit_log               (partitioned)
-- =============================================================================

-- ─────────────────────────────────────────────────────────────────────────────
-- 1. Extend the agencies table
-- ─────────────────────────────────────────────────────────────────────────────

ALTER TABLE agencies
    ADD COLUMN IF NOT EXISTS settings             JSONB       NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS locale               TEXT        NOT NULL DEFAULT 'en-KE',
    -- currency_code already exists; ALTER only if it doesn't
    ADD COLUMN IF NOT EXISTS subdomain            TEXT        UNIQUE,
    ADD COLUMN IF NOT EXISTS is_vat_registered    BOOLEAN     NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS is_wht_agent         BOOLEAN     NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS charge_vat           BOOLEAN     NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS vat_rate             NUMERIC(5,4) NOT NULL DEFAULT 0.1600,
    ADD COLUMN IF NOT EXISTS audit_level          TEXT        NOT NULL DEFAULT 'financial'
        CHECK (audit_level IN ('minimal', 'financial', 'full')),
    ADD COLUMN IF NOT EXISTS data_retention_years SMALLINT    NOT NULL DEFAULT 7;

-- ─────────────────────────────────────────────────────────────────────────────
-- 2. agency_custom_domains
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS agency_custom_domains (
    id              UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id       UUID        NOT NULL REFERENCES agencies (id) ON DELETE CASCADE,
    domain          TEXT        NOT NULL UNIQUE,   -- e.g. "portal.blissvilla.com"
    verified        BOOLEAN     NOT NULL DEFAULT false,
    cert_issued_at  TIMESTAMPTZ,
    cert_expires_at TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_custom_domains_agency
    ON agency_custom_domains (agency_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 3. agency_locale_strings  (per-agency i18n overrides)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS agency_locale_strings (
    id        UUID NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id UUID NOT NULL REFERENCES agencies (id) ON DELETE CASCADE,
    locale    TEXT NOT NULL,   -- e.g. 'sw-KE', 'fr-FR'
    key       TEXT NOT NULL,   -- e.g. 'invoice.footer', 'welcome.sms'
    value     TEXT NOT NULL,
    UNIQUE (agency_id, locale, key)
);

CREATE INDEX IF NOT EXISTS idx_locale_strings_agency
    ON agency_locale_strings (agency_id, locale);

-- ─────────────────────────────────────────────────────────────────────────────
-- 4. agency_compliance
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS agency_compliance (
    agency_id              UUID     NOT NULL PRIMARY KEY
                                    REFERENCES agencies (id) ON DELETE CASCADE,
    kra_pin                TEXT,
    vat_number             TEXT,
    wht_rate               NUMERIC(5,4) NOT NULL DEFAULT 0.10,
    county_code            TEXT,       -- 'NBI','MSA','NKR'
    privacy_policy_url     TEXT,
    consent_text           TEXT,
    dpa_consent_required   BOOLEAN NOT NULL DEFAULT true,
    aml_reporting_required BOOLEAN NOT NULL DEFAULT false,
    esign_provider         TEXT,       -- NULL = paper only
    data_residency_region  TEXT    NOT NULL DEFAULT 'af-south-1',
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ─────────────────────────────────────────────────────────────────────────────
-- 5. agency_security_policy
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS agency_security_policy (
    agency_id               UUID     NOT NULL PRIMARY KEY
                                     REFERENCES agencies (id) ON DELETE CASCADE,
    mfa_requirement         TEXT     NOT NULL DEFAULT 'optional'
        CHECK (mfa_requirement IN ('none', 'admins_only', 'all_users')),
    session_idle_minutes    SMALLINT NOT NULL DEFAULT 480,
    session_absolute_hours  SMALLINT NOT NULL DEFAULT 24,
    password_min_length     SMALLINT NOT NULL DEFAULT 8,
    password_require_upper  BOOLEAN  NOT NULL DEFAULT false,
    password_require_symbol BOOLEAN  NOT NULL DEFAULT false,
    password_expiry_days    SMALLINT,           -- NULL = never
    sso_enabled             BOOLEAN  NOT NULL DEFAULT false,
    sso_provider            TEXT,               -- 'google','azure_ad','okta'
    sso_metadata_url        TEXT,
    ip_allowlist            JSONB    NOT NULL DEFAULT '[]',
    audit_retention_days    INT      NOT NULL DEFAULT 365,
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ─────────────────────────────────────────────────────────────────────────────
-- 6. custom_roles
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS custom_roles (
    id          UUID    NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id   UUID    NOT NULL REFERENCES agencies (id) ON DELETE CASCADE,
    name        TEXT    NOT NULL,
    -- e.g. ["agreements:read","payments:write","reports:read","staff:manage"]
    permissions JSONB   NOT NULL DEFAULT '[]',
    is_system   BOOLEAN NOT NULL DEFAULT false,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agency_id, name)
);

CREATE INDEX IF NOT EXISTS idx_custom_roles_agency
    ON custom_roles (agency_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 7. agency_plan_overrides  (sales one-offs — single feature per agency)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS agency_plan_overrides (
    id          UUID    NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id   UUID    NOT NULL REFERENCES agencies (id) ON DELETE CASCADE,
    feature_key TEXT    NOT NULL,
    value       JSONB   NOT NULL,   -- true, false, or numeric limit
    reason      TEXT    NOT NULL,
    expires_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agency_id, feature_key)
);

CREATE INDEX IF NOT EXISTS idx_plan_overrides_agency
    ON agency_plan_overrides (agency_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 8. audit_log  (immutable, partitioned monthly)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS audit_log (
    id          UUID        NOT NULL DEFAULT uuidv7(),
    agency_id   UUID        NOT NULL,
    actor_id    UUID,                   -- NULL = system job
    actor_role  TEXT,
    action      TEXT        NOT NULL,   -- e.g. 'agreement.rent_amount.updated'
    entity_type TEXT        NOT NULL,
    entity_id   UUID        NOT NULL,
    old_data    JSONB,
    new_data    JSONB,
    ip_address  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

-- Seed initial partitions (current month + two ahead).
-- The Apalis tax_worker / archival_job creates future partitions on the 25th.
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

CREATE INDEX IF NOT EXISTS idx_audit_agency_entity
    ON audit_log (agency_id, entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_audit_actor
    ON audit_log (agency_id, actor_id, created_at DESC);