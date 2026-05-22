-- =============================================================================
-- migrations/agency/0003_customisation.sql
--
-- Multi-tenant customisation layer — agency-schema side.
--
-- New tables (in FK-safe order):
--   1.  late_fee_policy
--   2.  agency_integrations
--   3.  agency_notification_toggles
--   4.  notification_templates
--   5.  workflow_rules
--   6.  workflow_executions
--   7.  report_configs
--   8.  scheduled_reports
--   9.  onboarding_checklists
--  10.  agency_onboarding_progress
--  11.  data_retention_policy
--  12.  archival_log
--  13.  agency_feature_flags        (discovery log / rolling feature flags)
-- =============================================================================

-- ─────────────────────────────────────────────────────────────────────────────
-- 1. late_fee_policy
--    Agencies can define multiple policies (one per property type) and assign
--    them per agreement. A partial unique index enforces one default per agency.
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS late_fee_policy (
    id                UUID           NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id         UUID           NOT NULL,   -- references platform agencies.id (no FK across DBs)
    name              TEXT           NOT NULL,
    is_default        BOOLEAN        NOT NULL DEFAULT false,
    grace_period_days SMALLINT       NOT NULL DEFAULT 5,
    fee_type          TEXT           NOT NULL CHECK (fee_type IN ('flat', 'percent')),
    flat_amount_kes   NUMERIC(12,2),
    percent_of_rent   NUMERIC(5,4),   -- 0.05 = 5%
    compounding       BOOLEAN        NOT NULL DEFAULT false,
    max_fee_kes       NUMERIC(12,2),
    created_at        TIMESTAMPTZ    NOT NULL DEFAULT now(),
    UNIQUE (agency_id, name)
);

-- Only one default per agency.
CREATE UNIQUE INDEX IF NOT EXISTS idx_late_fee_one_default
    ON late_fee_policy (agency_id) WHERE is_default = true;

CREATE INDEX IF NOT EXISTS idx_late_fee_agency
    ON late_fee_policy (agency_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 2. agency_integrations
--    Credentials are AES-256-GCM encrypted at the application layer before INSERT.
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS agency_integrations (
    id            UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id     UUID        NOT NULL,
    provider_type TEXT        NOT NULL,   -- 'sms','email','payment','storage','esign'
    provider_key  TEXT        NOT NULL,   -- 'africas_talking','mpesa','sendgrid'
    is_active     BOOLEAN     NOT NULL DEFAULT true,
    -- AES-256-GCM encrypted at the application layer before INSERT
    credentials   JSONB       NOT NULL DEFAULT '{}',
    -- Non-secret configuration (shortcodes, bucket names, etc.)
    settings      JSONB       NOT NULL DEFAULT '{}',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agency_id, provider_type, provider_key)
);

CREATE INDEX IF NOT EXISTS idx_integrations_active
    ON agency_integrations (agency_id, provider_type) WHERE is_active = true;

-- ─────────────────────────────────────────────────────────────────────────────
-- 3. agency_notification_toggles
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS agency_notification_toggles (
    agency_id        UUID    NOT NULL,
    event_key        TEXT    NOT NULL,   -- 'rent.overdue', 'lease.expiry_notice'
    sms_enabled      BOOLEAN NOT NULL DEFAULT true,
    email_enabled    BOOLEAN NOT NULL DEFAULT true,
    whatsapp_enabled BOOLEAN NOT NULL DEFAULT false,
    PRIMARY KEY (agency_id, event_key)
);

-- ─────────────────────────────────────────────────────────────────────────────
-- 4. notification_templates  (MiniJinja strings stored in DB)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS notification_templates (
    id         UUID    NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id  UUID    NOT NULL,
    channel    TEXT    NOT NULL CHECK (channel IN ('sms', 'email', 'whatsapp')),
    event_key  TEXT    NOT NULL,
    locale     TEXT    NOT NULL DEFAULT 'en',
    subject    TEXT,               -- email only
    body       TEXT    NOT NULL,   -- MiniJinja template string
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agency_id, channel, event_key, locale)
);

CREATE INDEX IF NOT EXISTS idx_notif_templates_agency
    ON notification_templates (agency_id, channel, event_key);

-- ─────────────────────────────────────────────────────────────────────────────
-- 5. workflow_rules  (JSONLogic conditions, action arrays)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS workflow_rules (
    id           UUID    NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id    UUID    NOT NULL,
    name         TEXT    NOT NULL,
    event_type   TEXT    NOT NULL,   -- 'rent.overdue','agreement.created'
    is_active    BOOLEAN NOT NULL DEFAULT true,
    offset_hours INT     NOT NULL DEFAULT 0,   -- negative = before event
    -- JSONLogic condition: {">=": [{"var": "days_overdue"}, 3]}
    conditions   JSONB   NOT NULL DEFAULT '{}',
    -- Array of {type, params}: [{"type":"send_sms","params":{"template":"overdue_3d"}}]
    actions      JSONB   NOT NULL DEFAULT '[]',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agency_id, name)
);

CREATE INDEX IF NOT EXISTS idx_workflow_rules_agency_event
    ON workflow_rules (agency_id, event_type) WHERE is_active = true;

-- ─────────────────────────────────────────────────────────────────────────────
-- 6. workflow_executions
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS workflow_executions (
    id          UUID    NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    rule_id     UUID    NOT NULL REFERENCES workflow_rules (id),
    agency_id   UUID    NOT NULL,
    entity_type TEXT    NOT NULL,
    entity_id   UUID    NOT NULL,
    status      TEXT    NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'running', 'completed', 'failed')),
    error       TEXT,
    executed_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_workflow_exec_rule
    ON workflow_executions (rule_id, status);
CREATE INDEX IF NOT EXISTS idx_workflow_exec_entity
    ON workflow_executions (agency_id, entity_type, entity_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 7. report_configs
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS report_configs (
    id                 UUID    NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id          UUID    NOT NULL,
    report_type        TEXT    NOT NULL,
    name               TEXT    NOT NULL,
    columns            JSONB   NOT NULL DEFAULT '[]',
    filters            JSONB   NOT NULL DEFAULT '{}',
    highlight_rules    JSONB   NOT NULL DEFAULT '[]',
    fiscal_year_start  SMALLINT NOT NULL DEFAULT 1,   -- 1 = Jan
    coa_mapping        JSONB   NOT NULL DEFAULT '{}', -- account grouping overrides
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agency_id, report_type, name)
);

CREATE INDEX IF NOT EXISTS idx_report_configs_agency
    ON report_configs (agency_id, report_type);

-- ─────────────────────────────────────────────────────────────────────────────
-- 8. scheduled_reports
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS scheduled_reports (
    id            UUID    NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    config_id     UUID    NOT NULL REFERENCES report_configs (id) ON DELETE CASCADE,
    agency_id     UUID    NOT NULL,
    frequency     TEXT    NOT NULL CHECK (frequency IN ('daily', 'weekly', 'monthly')),
    day_of_week   SMALLINT,
    day_of_month  SMALLINT,
    recipients    JSONB   NOT NULL DEFAULT '[]',
    export_format TEXT    NOT NULL DEFAULT 'pdf'
        CHECK (export_format IN ('pdf', 'xlsx', 'csv')),
    last_sent_at  TIMESTAMPTZ,
    is_active     BOOLEAN NOT NULL DEFAULT true,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_scheduled_reports_agency
    ON scheduled_reports (agency_id) WHERE is_active = true;

-- ─────────────────────────────────────────────────────────────────────────────
-- 9. onboarding_checklists
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS onboarding_checklists (
    id         UUID NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    -- NULL = applies to all plans
    plan_key   TEXT,
    name       TEXT NOT NULL,
    -- [{"key":"add_property","title":"Add your first property","required":true}, ...]
    steps      JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ─────────────────────────────────────────────────────────────────────────────
-- 10. agency_onboarding_progress
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS agency_onboarding_progress (
    agency_id        UUID NOT NULL PRIMARY KEY,
    checklist_id     UUID NOT NULL REFERENCES onboarding_checklists (id),
    completed_steps  JSONB NOT NULL DEFAULT '[]',
    dismissed_at     TIMESTAMPTZ,
    completed_at     TIMESTAMPTZ,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ─────────────────────────────────────────────────────────────────────────────
-- 11. data_retention_policy
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS data_retention_policy (
    agency_id              UUID     NOT NULL PRIMARY KEY,
    financial_retain_years SMALLINT NOT NULL DEFAULT 7,
    document_retain_years  SMALLINT NOT NULL DEFAULT 7,
    pii_erasure_enabled    BOOLEAN  NOT NULL DEFAULT true,
    cold_storage_bucket    TEXT,    -- NULL = keep in Postgres
    archival_trigger       TEXT     NOT NULL DEFAULT 'agreement_expiry'
        CHECK (archival_trigger IN ('agreement_expiry', 'calendar_year', 'manual')),
    archive_offset_months  SMALLINT NOT NULL DEFAULT 6,
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ─────────────────────────────────────────────────────────────────────────────
-- 12. archival_log
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS archival_log (
    id           UUID NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id    UUID NOT NULL,
    entity_type  TEXT NOT NULL,
    entity_id    UUID NOT NULL,
    action       TEXT NOT NULL CHECK (action IN ('archived', 'anonymised', 'deleted')),
    archive_key  TEXT,   -- S3 object key
    performed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    performed_by UUID
);

CREATE INDEX IF NOT EXISTS idx_archival_log_agency
    ON archival_log (agency_id, entity_type, performed_at DESC);

-- ─────────────────────────────────────────────────────────────────────────────
-- 13. agency_feature_flags  (rolling discovery log)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS agency_feature_flags (
    id         UUID    NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id  UUID    NOT NULL,
    key        TEXT    NOT NULL,
    enabled    BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agency_id, key)
);

CREATE INDEX IF NOT EXISTS idx_feature_flags_key
    ON agency_feature_flags (key) WHERE enabled = true;

-- ─────────────────────────────────────────────────────────────────────────────
-- updated_at triggers (reuse the existing tenant_set_updated_at function)
-- ─────────────────────────────────────────────────────────────────────────────

DO $$
DECLARE
    tbl TEXT;
BEGIN
    FOREACH tbl IN ARRAY ARRAY[
        'agency_integrations',
        'agency_feature_flags',
        'agency_onboarding_progress',
        'data_retention_policy'
    ]
    LOOP
        EXECUTE format('
            CREATE TRIGGER trg_%s_updated_at
            BEFORE UPDATE ON %s
            FOR EACH ROW EXECUTE FUNCTION tenant_set_updated_at();
        ', tbl, tbl);
    END LOOP;
END;
$$;