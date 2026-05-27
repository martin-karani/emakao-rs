-- =============================================================================
--
-- Agency schema — full baseline (one schema per tenant inside emakao_agency).
--
-- Tables (in FK-safe order):
--   1.  properties
--   2.  units
--   3.  residents
--   4.  agreements
--   5.  owners
--   6.  property_owners
--   7.  payment_claims
--   8.  ledger_entries
--   9.  vendors
--  10.  caretakers
--  11.  work_orders
--  12.  work_order_comments
--  13.  work_order_activity
--  14.  utility_meters
--  15.  meter_readings
--  16.  utility_bills
--  17.  applicants
--  18.  disbursements
--  19.  inspections
--  20.  invoices
--  21.  conversations
--  22.  messages
--  23.  documents
--  24.  rent_charges
--  25.  late_fee_charges
--  26.  rent_reminders_sent
--  27.  accounts
--  28.  journal_entries
--  29.  journal_lines
--  30.  bank_statements
--  31.  bank_statement_lines
--  32.  tax_obligations
--  33.  owner_annual_rental_income
--  34.  late_fee_policy
--  35.  agency_integrations
--  36.  agency_notification_toggles
--  37.  notification_templates
--  38.  workflow_rules
--  39.  workflow_executions
--  40.  report_configs
--  41.  scheduled_reports
--  42.  onboarding_checklists
--  43.  agency_onboarding_progress
--  44.  data_retention_policy
--  45.  archival_log
--  46.  agency_feature_flags
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

CREATE TYPE property_type AS ENUM (
    'residential',
    'commercial',
    'community',
    'student',
    'affordable_housing'
);

CREATE TYPE unit_status AS ENUM (
    'vacant',
    'occupied',
    'maintenance',
    'reserved',
    'inactive'
);

CREATE TYPE portal_status AS ENUM (
    'invited',
    'active',
    'suspended'
);

CREATE TYPE agreement_status AS ENUM (
    'draft',
    'active',
    'expired',
    'terminated',
    'pending_renewal'
);

CREATE TYPE billing_frequency AS ENUM (
    'monthly',
    'quarterly',
    'semi_annual',
    'annual'
);

CREATE TYPE payment_method_type AS ENUM (
    'mpesa_stk',
    'mpesa_paybill',
    'mpesa_till',
    'bank_transfer',
    'cash'
);

CREATE TYPE claim_status AS ENUM (
    'pending_review',
    'approved',
    'rejected'
);

CREATE TYPE ledger_entry_type AS ENUM (
    'rent',
    'deposit',
    'hoa_dues',
    'cam_charge',
    'utility',
    'maintenance_charge',
    'late_fee',
    'legal_fee',
    'penalty',
    'payment_mpesa',
    'payment_bank',
    'payment_cash',
    'deposit_refund',
    'credit_note',
    'waiver',
    'disbursement',
    'journal_adjustment'
);

CREATE TYPE work_order_status AS ENUM (
    'open',
    'in_progress',
    'completed',
    'cancelled'
);

CREATE TYPE work_order_priority AS ENUM (
    'low',
    'medium',
    'high',
    'emergency'
);

CREATE TYPE work_order_category AS ENUM (
    'plumbing',
    'electrical',
    'structural',
    'hvac',
    'appliance',
    'painting',
    'cleaning',
    'security',
    'landscaping',
    'pest_control',
    'general'
);

CREATE TYPE work_order_reporter_type AS ENUM (
    'staff',
    'resident',
    'caretaker',
    'owner',
    'vendor'
);

CREATE TYPE work_order_comment_author_type AS ENUM (
    'staff',
    'resident',
    'caretaker',
    'owner',
    'vendor'
);

CREATE TYPE vendor_status AS ENUM (
    'active',
    'inactive',
    'blacklisted'
);

CREATE TYPE meter_type AS ENUM (
    'electricity',
    'water',
    'gas'
);

CREATE TYPE billing_mode AS ENUM (
    'prepaid',
    'postpaid'
);

CREATE TYPE utility_bill_status AS ENUM (
    'draft',
    'issued',
    'paid',
    'overdue'
);

CREATE TYPE application_status AS ENUM (
    'submitted',
    'under_review',
    'approved',
    'rejected',
    'withdrawn'
);

CREATE TYPE disbursement_method AS ENUM (
    'bank_transfer',
    'mpesa_b2c',
    'cheque'
);

CREATE TYPE disbursement_status AS ENUM (
    'pending',
    'processing',
    'completed',
    'failed'
);

CREATE TYPE inspection_type AS ENUM (
    'move_in',
    'move_out',
    'routine',
    'emergency'
);

CREATE TYPE inspection_status AS ENUM (
    'scheduled',
    'in_progress',
    'completed',
    'cancelled'
);

CREATE TYPE message_sender_type AS ENUM (
    'staff',
    'resident',
    'owner',
    'system'
);

CREATE TYPE invoice_status AS ENUM (
    'draft',
    'sent',
    'paid',
    'overdue',
    'void'
);

CREATE TYPE document_type AS ENUM (
    'lease_agreement',
    'noc',
    'inspection_form',
    'utility_bill',
    'receipt',
    'id_document',
    'photo',
    'other'
);

CREATE TYPE account_category AS ENUM (
    'asset',
    'liability',
    'equity',
    'revenue',
    'expense'
);

CREATE TYPE journal_entry_status AS ENUM (
    'draft',
    'posted',
    'voided'
);

CREATE TYPE tax_obligation_type AS ENUM ('mri', 'vat', 'wht');

CREATE TYPE tax_obligation_status AS ENUM (
    'pending',
    'filed',
    'paid',
    'overdue',
    'nil_filed'
);

CREATE TYPE mri_regime AS ENUM (
    'exempt',
    'mri',
    'normal_income_tax',
    'elected_normal'
);

-- ─────────────────────────────────────────────────────────────────────────────
-- 1. properties
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE properties (
    id               UUID          NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id        UUID          NOT NULL,
    name             TEXT          NOT NULL CHECK (length(trim(name)) > 0),
    address          TEXT          NOT NULL CHECK (length(trim(address)) > 0),
    city             TEXT          NOT NULL DEFAULT '',
    country_code     CHAR(2)       NOT NULL DEFAULT 'KE',
    property_type    property_type NOT NULL,
    config           JSONB         NOT NULL DEFAULT '{}',
    work_order_prefix VARCHAR(8)   NOT NULL DEFAULT '',
        CHECK (work_order_prefix ~ '^[A-Z][A-Z0-9]{1,7}$'),
    work_order_seq   INT           NOT NULL DEFAULT 0 CHECK (work_order_seq >= 0),
    created_by       UUID          NOT NULL,
    created_at       TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ   NOT NULL DEFAULT now(),
    UNIQUE (work_order_prefix)
);

CREATE INDEX idx_properties_agency     ON properties (agency_id);
CREATE INDEX idx_properties_type       ON properties (agency_id, property_type);
CREATE INDEX idx_properties_created_by ON properties (created_by);

-- ─────────────────────────────────────────────────────────────────────────────
-- 2. units
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE units (
    id              UUID          NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    property_id     UUID          NOT NULL REFERENCES properties (id) ON DELETE CASCADE,
    parent_unit_id  UUID          REFERENCES units (id) ON DELETE SET NULL,
    unit_number     TEXT          NOT NULL CHECK (length(trim(unit_number)) > 0),
    floor           INTEGER       CHECK (floor >= -10),
    size_sqm        NUMERIC(10,2) CHECK (size_sqm > 0),
    bedrooms        SMALLINT      CHECK (bedrooms >= 0),
    bathrooms       SMALLINT      CHECK (bathrooms >= 0),
    rent_amount_kes NUMERIC(14,2) NOT NULL DEFAULT 0 CHECK (rent_amount_kes >= 0),
    deposit_kes     NUMERIC(14,2) NOT NULL DEFAULT 0 CHECK (deposit_kes >= 0),
    status          unit_status   NOT NULL DEFAULT 'vacant',
    description     TEXT,
    photos          JSONB         NOT NULL DEFAULT '[]',
    created_at      TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ   NOT NULL DEFAULT now(),
    UNIQUE (property_id, unit_number)
);

CREATE INDEX idx_units_property ON units (property_id);
CREATE INDEX idx_units_status   ON units (property_id, status);
CREATE INDEX idx_units_parent   ON units (parent_unit_id) WHERE parent_unit_id IS NOT NULL;

-- ─────────────────────────────────────────────────────────────────────────────
-- 3. residents
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE residents (
    id            UUID          NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    user_id       UUID,
    first_name    TEXT          NOT NULL CHECK (length(trim(first_name)) > 0),
    last_name     TEXT          NOT NULL CHECK (length(trim(last_name)) > 0),
    email         TEXT          CHECK (email = lower(email)),
    phone         TEXT,
    national_id   TEXT,
    portal_status portal_status NOT NULL DEFAULT 'invited',
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ   NOT NULL DEFAULT now()
);

CREATE INDEX idx_residents_email   ON residents (email)   WHERE email IS NOT NULL;
CREATE INDEX idx_residents_user_id ON residents (user_id) WHERE user_id IS NOT NULL;
CREATE INDEX idx_residents_phone   ON residents (phone)   WHERE phone IS NOT NULL;

-- ─────────────────────────────────────────────────────────────────────────────
-- 4. agreements
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE agreements (
    id                UUID              NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    property_id       UUID              NOT NULL REFERENCES properties (id) ON DELETE RESTRICT,
    unit_id           UUID              NOT NULL REFERENCES units      (id) ON DELETE RESTRICT,
    resident_id       UUID              NOT NULL REFERENCES residents  (id) ON DELETE RESTRICT,
    start_date        DATE              NOT NULL,
    end_date          DATE              CHECK (end_date > start_date),
    rent_amount_kes   NUMERIC(14,2)     NOT NULL CHECK (rent_amount_kes > 0),
    deposit_kes       NUMERIC(14,2)     NOT NULL DEFAULT 0 CHECK (deposit_kes >= 0),
    billing_frequency billing_frequency NOT NULL DEFAULT 'monthly',
    status            agreement_status  NOT NULL DEFAULT 'draft',
    notes             TEXT,
    created_at        TIMESTAMPTZ       NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ       NOT NULL DEFAULT now()
);

CREATE INDEX idx_agreements_unit     ON agreements (unit_id, status);
CREATE INDEX idx_agreements_resident ON agreements (resident_id);
CREATE INDEX idx_agreements_property ON agreements (property_id);
CREATE UNIQUE INDEX idx_agreements_active_unit ON agreements (unit_id) WHERE status = 'active';

-- ─────────────────────────────────────────────────────────────────────────────
-- 5. owners
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE owners (
    id            UUID          NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    user_id       UUID,
    first_name    TEXT          NOT NULL CHECK (length(trim(first_name)) > 0),
    last_name     TEXT          NOT NULL CHECK (length(trim(last_name)) > 0),
    email         TEXT          CHECK (email = lower(email)),
    phone         TEXT,
    company_name  TEXT,
    kra_pin       TEXT          CHECK (kra_pin ~ '^[A-Z]\d{9}[A-Z]$' OR kra_pin IS NULL),
    bank_name     TEXT,
    bank_account  TEXT,
    mpesa_number  TEXT,
    portal_status portal_status NOT NULL DEFAULT 'invited',
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ   NOT NULL DEFAULT now()
);

CREATE INDEX idx_owners_email   ON owners (email)   WHERE email IS NOT NULL;
CREATE INDEX idx_owners_user_id ON owners (user_id) WHERE user_id IS NOT NULL;

-- ─────────────────────────────────────────────────────────────────────────────
-- 6. property_owners
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE property_owners (
    property_id       UUID         NOT NULL REFERENCES properties (id) ON DELETE CASCADE,
    owner_id          UUID         NOT NULL REFERENCES owners     (id) ON DELETE CASCADE,
    ownership_percent NUMERIC(5,2) NOT NULL DEFAULT 100
        CHECK (ownership_percent > 0 AND ownership_percent <= 100),
    PRIMARY KEY (property_id, owner_id)
);

CREATE INDEX idx_property_owners_owner ON property_owners (owner_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 7. payment_claims
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE payment_claims (
    id               UUID                NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    property_id      UUID                NOT NULL REFERENCES properties (id) ON DELETE RESTRICT,
    agreement_id     UUID                REFERENCES agreements (id) ON DELETE SET NULL,
    resident_id      UUID                REFERENCES residents  (id) ON DELETE SET NULL,
    method_type      payment_method_type NOT NULL,
    amount_kes       NUMERIC(14,2)       NOT NULL CHECK (amount_kes > 0),
    reference_code   TEXT,
    proof_url        TEXT,
    notes            TEXT,
    status           claim_status        NOT NULL DEFAULT 'pending_review',
    reviewed_by      UUID,
    reviewed_at      TIMESTAMPTZ,
    review_notes     TEXT,
    rejection_reason TEXT,
    ledger_entry_id  UUID,
    submitted_by     UUID                NOT NULL,
    created_at       TIMESTAMPTZ         NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ         NOT NULL DEFAULT now()
);

CREATE INDEX idx_payment_claims_property     ON payment_claims (property_id, status);
CREATE INDEX idx_payment_claims_agreement    ON payment_claims (agreement_id) WHERE agreement_id IS NOT NULL;
CREATE INDEX idx_payment_claims_resident     ON payment_claims (resident_id)  WHERE resident_id IS NOT NULL;
CREATE INDEX idx_payment_claims_submitted_by ON payment_claims (submitted_by);

-- ─────────────────────────────────────────────────────────────────────────────
-- 8. ledger_entries
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE ledger_entries (
    id            UUID              NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agreement_id  UUID              REFERENCES agreements (id) ON DELETE SET NULL,
    unit_id       UUID              REFERENCES units      (id) ON DELETE SET NULL,
    resident_id   UUID              REFERENCES residents  (id) ON DELETE SET NULL,
    owner_id      UUID,
    entry_type    ledger_entry_type NOT NULL,
    amount_kes    NUMERIC(14,2)     NOT NULL,
    description   TEXT              NOT NULL CHECK (length(trim(description)) > 0),
    external_ref  TEXT,
    mpesa_receipt TEXT,
    period_start  DATE,
    period_end    DATE,
    posted_by     UUID              NOT NULL,
    posted_at     TIMESTAMPTZ       NOT NULL DEFAULT now(),
    is_reconciled BOOLEAN           NOT NULL DEFAULT false,
    metadata      JSONB             NOT NULL DEFAULT '{}',
    CONSTRAINT ledger_period_valid
        CHECK (period_end IS NULL OR period_start IS NULL OR period_end >= period_start)
);

CREATE INDEX idx_ledger_agreement    ON ledger_entries (agreement_id, posted_at DESC) WHERE agreement_id IS NOT NULL;
CREATE INDEX idx_ledger_unit         ON ledger_entries (unit_id)        WHERE unit_id IS NOT NULL;
CREATE INDEX idx_ledger_resident     ON ledger_entries (resident_id)    WHERE resident_id IS NOT NULL;
CREATE INDEX idx_ledger_type         ON ledger_entries (entry_type, posted_at DESC);
CREATE INDEX idx_ledger_mpesa        ON ledger_entries (mpesa_receipt)  WHERE mpesa_receipt IS NOT NULL;
CREATE INDEX idx_ledger_unreconciled ON ledger_entries (posted_at)      WHERE is_reconciled = false;

-- ─────────────────────────────────────────────────────────────────────────────
-- 9. vendors
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE vendors (
    id            UUID          NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id     UUID          NOT NULL,
    user_id       UUID,
    name          TEXT          NOT NULL CHECK (length(trim(name)) > 0),
    contact_name  TEXT,
    email         TEXT          CHECK (email = lower(email)),
    phone         TEXT,
    speciality    TEXT,
    status        vendor_status NOT NULL DEFAULT 'active',
    portal_status portal_status NOT NULL DEFAULT 'invited',
    notes         TEXT,
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ   NOT NULL DEFAULT now()
);

CREATE INDEX idx_vendors_agency  ON vendors (agency_id, status);
CREATE INDEX idx_vendors_user_id ON vendors (user_id) WHERE user_id IS NOT NULL;

-- ─────────────────────────────────────────────────────────────────────────────
-- 10. caretakers
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE caretakers (
    id          UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    property_id UUID        NOT NULL REFERENCES properties (id) ON DELETE CASCADE,
    user_id     UUID,
    first_name  TEXT        NOT NULL CHECK (length(trim(first_name)) > 0),
    last_name   TEXT        NOT NULL CHECK (length(trim(last_name)) > 0),
    phone       TEXT,
    email       TEXT        CHECK (email = lower(email)),
    is_active   BOOLEAN     NOT NULL DEFAULT true,
    created_by  UUID        NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_caretakers_property ON caretakers (property_id, is_active);
CREATE INDEX idx_caretakers_user_id  ON caretakers (user_id) WHERE user_id IS NOT NULL;

-- ─────────────────────────────────────────────────────────────────────────────
-- 11. work_orders
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE work_orders (
    id                    UUID                     NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    property_id           UUID                     NOT NULL REFERENCES properties (id) ON DELETE RESTRICT,
    unit_id               UUID                     REFERENCES units      (id) ON DELETE SET NULL,
    vendor_id             UUID                     REFERENCES vendors    (id) ON DELETE SET NULL,
    work_order_number     INT                      NOT NULL,
    code                  TEXT                     NOT NULL UNIQUE,
        CHECK (code ~ '^[A-Z][A-Z0-9]{1,7}-[0-9]+$'),
    title                 TEXT                     NOT NULL CHECK (length(trim(title)) > 0),
    description           TEXT,
    category              work_order_category      NOT NULL DEFAULT 'general',
    status                work_order_status        NOT NULL DEFAULT 'open',
    priority              work_order_priority      NOT NULL DEFAULT 'medium',
    reported_by           UUID                     NOT NULL,
    reporter_type         work_order_reporter_type NOT NULL DEFAULT 'staff',
    reporter_resident_id  UUID                     REFERENCES residents  (id) ON DELETE SET NULL,
    reporter_caretaker_id UUID                     REFERENCES caretakers (id) ON DELETE SET NULL,
    assigned_to           UUID,
    assigned_caretaker_id UUID                     REFERENCES caretakers (id) ON DELETE SET NULL,
    due_date              DATE,
    scheduled_at          TIMESTAMPTZ,
    started_at            TIMESTAMPTZ,
    completed_at          TIMESTAMPTZ,
    estimated_cost_kes    NUMERIC(14,2)            CHECK (estimated_cost_kes >= 0),
    actual_cost_kes       NUMERIC(14,2)            CHECK (actual_cost_kes >= 0),
    is_tenant_visible     BOOLEAN                  NOT NULL DEFAULT true,
    internal_notes        TEXT,
    attachments           JSONB                    NOT NULL DEFAULT '[]',
    created_at            TIMESTAMPTZ              NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ              NOT NULL DEFAULT now()
);

CREATE INDEX idx_work_orders_property      ON work_orders (property_id, status);
CREATE INDEX idx_work_orders_code          ON work_orders (code);
CREATE INDEX idx_work_orders_unit          ON work_orders (unit_id)              WHERE unit_id IS NOT NULL;
CREATE INDEX idx_work_orders_vendor        ON work_orders (vendor_id)            WHERE vendor_id IS NOT NULL;
CREATE INDEX idx_work_orders_assigned      ON work_orders (assigned_to)          WHERE assigned_to IS NOT NULL;
CREATE INDEX idx_work_orders_caretaker     ON work_orders (assigned_caretaker_id) WHERE assigned_caretaker_id IS NOT NULL;
CREATE INDEX idx_work_orders_reporter_res  ON work_orders (reporter_resident_id) WHERE reporter_resident_id IS NOT NULL;
CREATE INDEX idx_work_orders_due           ON work_orders (due_date)
    WHERE status NOT IN ('completed', 'cancelled') AND due_date IS NOT NULL;
CREATE INDEX idx_work_orders_open_priority ON work_orders (priority, created_at DESC)
    WHERE status NOT IN ('completed', 'cancelled');

-- ─────────────────────────────────────────────────────────────────────────────
-- 12. work_order_comments
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE work_order_comments (
    id                  UUID                           NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    work_order_id       UUID                           NOT NULL REFERENCES work_orders (id) ON DELETE CASCADE,
    parent_comment_id   UUID                           REFERENCES work_order_comments (id) ON DELETE CASCADE,
    author_id           UUID                           NOT NULL,
    author_type         work_order_comment_author_type NOT NULL DEFAULT 'staff',
    author_resident_id  UUID                           REFERENCES residents  (id) ON DELETE SET NULL,
    author_caretaker_id UUID                           REFERENCES caretakers (id) ON DELETE SET NULL,
    body                TEXT                           NOT NULL CHECK (length(trim(body)) > 0),
    is_internal         BOOLEAN                        NOT NULL DEFAULT false,
    attachments         JSONB                          NOT NULL DEFAULT '[]',
    is_edited           BOOLEAN                        NOT NULL DEFAULT false,
    created_at          TIMESTAMPTZ                    NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ                    NOT NULL DEFAULT now()
);

CREATE INDEX idx_wo_comments_work_order ON work_order_comments (work_order_id, created_at ASC);
CREATE INDEX idx_wo_comments_parent     ON work_order_comments (parent_comment_id) WHERE parent_comment_id IS NOT NULL;
CREATE INDEX idx_wo_comments_author     ON work_order_comments (author_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 13. work_order_activity
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE work_order_activity (
    id            UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    work_order_id UUID        NOT NULL REFERENCES work_orders (id) ON DELETE CASCADE,
    actor_id      UUID        NOT NULL,
    actor_type    work_order_comment_author_type NOT NULL DEFAULT 'staff',
    event_type    TEXT        NOT NULL CHECK (length(trim(event_type)) > 0),
    payload       JSONB       NOT NULL DEFAULT '{}',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_wo_activity_work_order ON work_order_activity (work_order_id, created_at ASC);

-- ─────────────────────────────────────────────────────────────────────────────
-- 14. utility_meters
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE utility_meters (
    id            UUID          NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    unit_id       UUID          NOT NULL REFERENCES units (id) ON DELETE CASCADE,
    meter_type    meter_type    NOT NULL,
    billing_mode  billing_mode  NOT NULL,
    meter_number  TEXT          NOT NULL CHECK (length(trim(meter_number)) > 0),
    rate_per_unit NUMERIC(10,4) NOT NULL CHECK (rate_per_unit > 0),
    is_active     BOOLEAN       NOT NULL DEFAULT true,
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT now(),
    UNIQUE (unit_id, meter_type)
);

CREATE INDEX idx_utility_meters_unit ON utility_meters (unit_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 15. meter_readings
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE meter_readings (
    id            UUID          NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    meter_id      UUID          NOT NULL REFERENCES utility_meters (id) ON DELETE CASCADE,
    reading_value NUMERIC(14,4) NOT NULL CHECK (reading_value >= 0),
    read_at       TIMESTAMPTZ   NOT NULL DEFAULT now(),
    recorded_by   UUID          NOT NULL
);

CREATE INDEX idx_meter_readings_meter ON meter_readings (meter_id, read_at DESC);

-- ─────────────────────────────────────────────────────────────────────────────
-- 16. utility_bills
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE utility_bills (
    id             UUID                NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    meter_id       UUID                NOT NULL REFERENCES utility_meters (id) ON DELETE RESTRICT,
    unit_id        UUID                NOT NULL REFERENCES units          (id) ON DELETE RESTRICT,
    units_consumed NUMERIC(14,4)       NOT NULL CHECK (units_consumed > 0),
    amount_kes     NUMERIC(14,2)       NOT NULL CHECK (amount_kes >= 0),
    status         utility_bill_status NOT NULL DEFAULT 'draft',
    billing_period DATE,
    created_at     TIMESTAMPTZ         NOT NULL DEFAULT now()
);

CREATE INDEX idx_utility_bills_unit  ON utility_bills (unit_id, status);
CREATE INDEX idx_utility_bills_meter ON utility_bills (meter_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 17. applicants
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE applicants (
    id                 UUID               NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id          UUID               NOT NULL,
    property_id        UUID               NOT NULL REFERENCES properties (id) ON DELETE RESTRICT,
    unit_id            UUID               REFERENCES units      (id) ON DELETE SET NULL,
    first_name         TEXT               NOT NULL CHECK (length(trim(first_name)) > 0),
    last_name          TEXT               NOT NULL CHECK (length(trim(last_name)) > 0),
    email              TEXT               NOT NULL CHECK (email = lower(email)),
    phone              TEXT,
    national_id        TEXT,
    monthly_income_kes NUMERIC(14,2)      CHECK (monthly_income_kes >= 0),
    employer           TEXT,
    status             application_status NOT NULL DEFAULT 'submitted',
    notes              TEXT,
    created_at         TIMESTAMPTZ        NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ        NOT NULL DEFAULT now()
);

CREATE INDEX idx_applicants_property ON applicants (property_id, status);
CREATE INDEX idx_applicants_email    ON applicants (email);

-- ─────────────────────────────────────────────────────────────────────────────
-- 18. disbursements
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE disbursements (
    id          UUID                NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id   UUID                NOT NULL,
    owner_id    UUID                NOT NULL REFERENCES owners     (id) ON DELETE RESTRICT,
    property_id UUID                NOT NULL REFERENCES properties (id) ON DELETE RESTRICT,
    amount_kes  NUMERIC(14,2)       NOT NULL CHECK (amount_kes > 0),
    method      disbursement_method NOT NULL,
    reference   TEXT,
    status      disbursement_status NOT NULL DEFAULT 'pending',
    period_start DATE               NOT NULL,
    period_end   DATE               NOT NULL CHECK (period_end >= period_start),
    notes       TEXT,
    created_by  UUID                NOT NULL,
    created_at  TIMESTAMPTZ         NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ         NOT NULL DEFAULT now()
);

CREATE INDEX idx_disbursements_agency   ON disbursements (agency_id);
CREATE INDEX idx_disbursements_owner    ON disbursements (owner_id);
CREATE INDEX idx_disbursements_property ON disbursements (property_id);
CREATE INDEX idx_disbursements_status   ON disbursements (status);

-- ─────────────────────────────────────────────────────────────────────────────
-- 19. inspections
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE inspections (
    id              UUID              NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    property_id     UUID              NOT NULL REFERENCES properties (id) ON DELETE RESTRICT,
    unit_id         UUID              NOT NULL REFERENCES units      (id) ON DELETE RESTRICT,
    agreement_id    UUID              REFERENCES agreements (id) ON DELETE SET NULL,
    inspection_type inspection_type   NOT NULL,
    status          inspection_status NOT NULL DEFAULT 'scheduled',
    scheduled_at    TIMESTAMPTZ       NOT NULL,
    completed_at    TIMESTAMPTZ,
    conducted_by    UUID,
    items           JSONB             NOT NULL DEFAULT '[]',
    summary_notes   TEXT,
    created_by      UUID              NOT NULL,
    created_at      TIMESTAMPTZ       NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ       NOT NULL DEFAULT now()
);

CREATE INDEX idx_inspections_unit      ON inspections (unit_id, status);
CREATE INDEX idx_inspections_scheduled ON inspections (scheduled_at) WHERE status = 'scheduled';
CREATE INDEX idx_inspections_agreement ON inspections (agreement_id) WHERE agreement_id IS NOT NULL;

-- ─────────────────────────────────────────────────────────────────────────────
-- 20. invoices
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE invoices (
    id                       UUID           NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id                UUID           NOT NULL,
    property_id              UUID           NOT NULL REFERENCES properties (id) ON DELETE RESTRICT,
    agreement_id             UUID           REFERENCES agreements (id) ON DELETE SET NULL,
    resident_id              UUID           REFERENCES residents  (id) ON DELETE SET NULL,
    owner_id                 UUID,
    invoice_number           TEXT           NOT NULL UNIQUE,
    line_items               JSONB          NOT NULL DEFAULT '[]',
    subtotal_kes             NUMERIC(14,2)  NOT NULL CHECK (subtotal_kes >= 0),
    tax_kes                  NUMERIC(14,2)  NOT NULL DEFAULT 0 CHECK (tax_kes >= 0),
    -- Per-tax-type breakdown
    mri_kes                  NUMERIC(18,2)  NOT NULL DEFAULT 0,
    vat_kes                  NUMERIC(18,2)  NOT NULL DEFAULT 0,
    wht_kes                  NUMERIC(18,2)  NOT NULL DEFAULT 0,
    net_payable_kes          NUMERIC(18,2),
    total_kes                NUMERIC(14,2)  NOT NULL CHECK (total_kes >= 0),
    due_date                 DATE           NOT NULL,
    status                   invoice_status NOT NULL DEFAULT 'draft',
    notes                    TEXT,
    voided_at                TIMESTAMPTZ,
    -- Tax metadata
    mri_regime               mri_regime,
    tax_period               DATE,
    tax_due_date             DATE,
    -- eTIMS fiscal device fields
    etims_cu_invoice_number  TEXT,
    etims_qr_code            TEXT,
    etims_accepted_at        TIMESTAMPTZ,
    created_by               UUID           NOT NULL,
    created_at               TIMESTAMPTZ    NOT NULL DEFAULT now(),
    updated_at               TIMESTAMPTZ    NOT NULL DEFAULT now()
);

CREATE INDEX idx_invoices_property  ON invoices (property_id, status);
CREATE INDEX idx_invoices_agreement ON invoices (agreement_id) WHERE agreement_id IS NOT NULL;
CREATE INDEX idx_invoices_resident  ON invoices (resident_id)  WHERE resident_id IS NOT NULL;
CREATE INDEX idx_invoices_due       ON invoices (due_date)     WHERE status NOT IN ('paid', 'void');

-- ─────────────────────────────────────────────────────────────────────────────
-- 21. conversations
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE conversations (
    id                   UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id            UUID        NOT NULL,
    participant_ids      UUID[]      NOT NULL DEFAULT '{}',
    entity_type          TEXT,
    entity_id            UUID,
    subject              TEXT,
    last_message_preview TEXT,
    last_message_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    is_archived          BOOLEAN     NOT NULL DEFAULT false,
    created_by           UUID        NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_conversations_agency       ON conversations (agency_id, last_message_at DESC);
CREATE INDEX idx_conversations_participants ON conversations USING GIN (participant_ids);

-- ─────────────────────────────────────────────────────────────────────────────
-- 22. messages
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE messages (
    id              UUID                NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    conversation_id UUID                NOT NULL REFERENCES conversations (id) ON DELETE CASCADE,
    sender_id       UUID                NOT NULL,
    sender_type     message_sender_type NOT NULL DEFAULT 'staff',
    body            TEXT                NOT NULL CHECK (length(trim(body)) > 0),
    attachments     JSONB               NOT NULL DEFAULT '[]',
    read_by         JSONB               NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ         NOT NULL DEFAULT now()
);

CREATE INDEX idx_messages_conversation ON messages (conversation_id, created_at ASC);

-- ─────────────────────────────────────────────────────────────────────────────
-- 23. documents
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE documents (
    id            UUID          NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id     UUID          NOT NULL,
    s3_key        TEXT          NOT NULL,
    file_name     TEXT          NOT NULL,
    mime_type     TEXT          NOT NULL DEFAULT 'application/octet-stream',
    size_bytes    BIGINT        NOT NULL DEFAULT 0 CHECK (size_bytes >= 0),
    document_type document_type NOT NULL DEFAULT 'other',
    title         TEXT,
    notes         TEXT,
    property_id   UUID          REFERENCES properties (id) ON DELETE SET NULL,
    unit_id       UUID,
    resident_id   UUID,
    agreement_id  UUID          REFERENCES agreements  (id) ON DELETE SET NULL,
    work_order_id UUID          REFERENCES work_orders (id) ON DELETE SET NULL,
    uploaded_by   UUID          NOT NULL,
    is_deleted    BOOLEAN       NOT NULL DEFAULT false,
    deleted_at    TIMESTAMPTZ,
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT now()
);

CREATE INDEX idx_documents_agency     ON documents (agency_id, created_at DESC) WHERE is_deleted = false;
CREATE INDEX idx_documents_property   ON documents (property_id)  WHERE property_id IS NOT NULL  AND is_deleted = false;
CREATE INDEX idx_documents_unit       ON documents (unit_id)      WHERE unit_id IS NOT NULL       AND is_deleted = false;
CREATE INDEX idx_documents_resident   ON documents (resident_id)  WHERE resident_id IS NOT NULL   AND is_deleted = false;
CREATE INDEX idx_documents_agreement  ON documents (agreement_id) WHERE agreement_id IS NOT NULL  AND is_deleted = false;
CREATE INDEX idx_documents_work_order ON documents (work_order_id) WHERE work_order_id IS NOT NULL AND is_deleted = false;
CREATE INDEX idx_documents_type       ON documents (agency_id, document_type) WHERE is_deleted = false;

-- ─────────────────────────────────────────────────────────────────────────────
-- 24-26. Billing scheduler tables
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE rent_charges (
    id              UUID          NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agreement_id    UUID          NOT NULL REFERENCES agreements (id) ON DELETE CASCADE,
    period_start    DATE          NOT NULL,
    period_end      DATE          NOT NULL CHECK (period_end >= period_start),
    amount_kes      NUMERIC(14,2) NOT NULL CHECK (amount_kes > 0),
    ledger_entry_id UUID,
    charged_at      TIMESTAMPTZ   NOT NULL DEFAULT now(),
    UNIQUE (agreement_id, period_start)
);

CREATE INDEX idx_rent_charges_agreement ON rent_charges (agreement_id, period_start DESC);

CREATE TABLE late_fee_charges (
    id              UUID          NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agreement_id    UUID          NOT NULL REFERENCES agreements (id) ON DELETE CASCADE,
    period_start    DATE          NOT NULL,
    amount_kes      NUMERIC(14,2) NOT NULL CHECK (amount_kes > 0),
    ledger_entry_id UUID,
    charged_at      TIMESTAMPTZ   NOT NULL DEFAULT now(),
    UNIQUE (agreement_id, period_start)
);

CREATE INDEX idx_late_fee_charges_agreement ON late_fee_charges (agreement_id, period_start DESC);

CREATE TABLE rent_reminders_sent (
    id           UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agreement_id UUID        NOT NULL REFERENCES agreements (id) ON DELETE CASCADE,
    period_start DATE        NOT NULL,
    channel      TEXT        NOT NULL CHECK (channel IN ('email', 'sms')),
    sent_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agreement_id, period_start, channel)
);

CREATE INDEX idx_rent_reminders_agreement ON rent_reminders_sent (agreement_id, period_start DESC);

-- ─────────────────────────────────────────────────────────────────────────────
-- 27. accounts  (double-entry chart of accounts)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE accounts (
    id             UUID             NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id      UUID             NOT NULL,
    code           TEXT             NOT NULL,
    name           TEXT             NOT NULL,
    account_type   account_category NOT NULL,
    balance        NUMERIC(18,2)    NOT NULL DEFAULT 0,
    is_system      BOOLEAN          NOT NULL DEFAULT false,
    vat_applicable BOOLEAN          NOT NULL DEFAULT false,
    vat_rate       NUMERIC(5,4)              DEFAULT 0.1600,
    created_at     TIMESTAMPTZ      NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ      NOT NULL DEFAULT now(),
    UNIQUE (agency_id, code)
);

CREATE INDEX idx_accounts_agency ON accounts (agency_id, account_type);

-- ─────────────────────────────────────────────────────────────────────────────
-- 28. journal_entries
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE journal_entries (
    id          UUID                 NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id   UUID                 NOT NULL,
    reference   TEXT                 NOT NULL,
    description TEXT,
    status      journal_entry_status NOT NULL DEFAULT 'draft',
    posted_by   UUID                 NOT NULL,
    posted_at   TIMESTAMPTZ          NOT NULL DEFAULT now(),
    created_at  TIMESTAMPTZ          NOT NULL DEFAULT now()
);

CREATE INDEX idx_journal_entries_agency ON journal_entries (agency_id, posted_at DESC);
CREATE INDEX idx_journal_entries_status ON journal_entries (agency_id, status);

-- ─────────────────────────────────────────────────────────────────────────────
-- 29. journal_lines
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE journal_lines (
    id               UUID          NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    journal_entry_id UUID          NOT NULL REFERENCES journal_entries (id) ON DELETE CASCADE,
    account_id       UUID          NOT NULL REFERENCES accounts (id),
    debit_kes        NUMERIC(18,2) NOT NULL DEFAULT 0 CHECK (debit_kes  >= 0),
    credit_kes       NUMERIC(18,2) NOT NULL DEFAULT 0 CHECK (credit_kes >= 0),
    description      TEXT,
    created_at       TIMESTAMPTZ   NOT NULL DEFAULT now(),
    -- Each line is either a pure debit or a pure credit, never both.
    CHECK (
        (debit_kes > 0 AND credit_kes = 0) OR
        (credit_kes > 0 AND debit_kes = 0)
    )
);

CREATE INDEX idx_journal_lines_entry   ON journal_lines (journal_entry_id);
CREATE INDEX idx_journal_lines_account ON journal_lines (account_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 30. bank_statements
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE bank_statements (
    id              UUID          NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id       UUID          NOT NULL,
    bank_name       TEXT          NOT NULL,
    account_number  TEXT          NOT NULL,
    statement_date  DATE          NOT NULL,
    opening_balance NUMERIC(18,2) NOT NULL,
    closing_balance NUMERIC(18,2) NOT NULL,
    created_by      UUID          NOT NULL,
    created_at      TIMESTAMPTZ   NOT NULL DEFAULT now()
);

CREATE INDEX idx_bank_statements_agency ON bank_statements (agency_id, statement_date DESC);

-- ─────────────────────────────────────────────────────────────────────────────
-- 31. bank_statement_lines
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE bank_statement_lines (
    id               UUID          NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    statement_id     UUID          NOT NULL REFERENCES bank_statements  (id) ON DELETE CASCADE,
    value_date       DATE          NOT NULL,
    description      TEXT          NOT NULL,
    amount           NUMERIC(18,2) NOT NULL,  -- positive = credit, negative = debit
    reference        TEXT,
    matched_entry_id UUID          REFERENCES journal_entries (id) ON DELETE SET NULL,
    created_at       TIMESTAMPTZ   NOT NULL DEFAULT now()
);

CREATE INDEX idx_bsl_statement    ON bank_statement_lines (statement_id);
CREATE INDEX idx_bsl_matched      ON bank_statement_lines (matched_entry_id) WHERE matched_entry_id IS NOT NULL;
CREATE INDEX idx_bsl_unreconciled ON bank_statement_lines (statement_id)     WHERE matched_entry_id IS NULL;

-- ─────────────────────────────────────────────────────────────────────────────
-- 32. tax_obligations  (Kenya MRI / VAT / WHT compliance)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE tax_obligations (
    id               UUID                  NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id        UUID                  NOT NULL,
    owner_id         UUID                  NOT NULL,
    property_id      UUID                  NOT NULL,
    agreement_id     UUID,
    obligation_type  tax_obligation_type   NOT NULL,
    -- First day of the calendar month, e.g. 2025-04-01 = April 2025
    tax_period       DATE                  NOT NULL,
    gross_amount_kes NUMERIC(18,2)         NOT NULL CHECK (gross_amount_kes >= 0),
    tax_kes          NUMERIC(18,2)         NOT NULL CHECK (tax_kes >= 0),
    tax_rate         NUMERIC(8,4)          NOT NULL,
    due_date         DATE                  NOT NULL,
    status           tax_obligation_status NOT NULL DEFAULT 'pending',
    kra_prn          TEXT,
    kra_ack_number   TEXT,
    filed_at         TIMESTAMPTZ,
    remitted_at      TIMESTAMPTZ,
    created_at       TIMESTAMPTZ           NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ           NOT NULL DEFAULT now(),

    UNIQUE (agency_id, owner_id, property_id, tax_period, obligation_type)
);

CREATE INDEX idx_tax_obligations_agency_period ON tax_obligations (agency_id, tax_period);
CREATE INDEX idx_tax_obligations_owner         ON tax_obligations (owner_id, tax_period);
CREATE INDEX idx_tax_obligations_status        ON tax_obligations (agency_id, status)
    WHERE status IN ('pending', 'overdue');
CREATE INDEX idx_tax_obligations_due_date      ON tax_obligations (due_date)
    WHERE status NOT IN ('paid', 'nil_filed');

-- ─────────────────────────────────────────────────────────────────────────────
-- 33. owner_annual_rental_income  (MRI regime tracker)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE owner_annual_rental_income (
    id                    UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id             UUID        NOT NULL,
    owner_id              UUID        NOT NULL,
    year                  SMALLINT    NOT NULL,
    total_gross_rent_kes  NUMERIC(18,2) NOT NULL DEFAULT 0,
    mri_regime            mri_regime  NOT NULL DEFAULT 'exempt',
    -- Set to TRUE when the owner has formally elected the normal income-tax regime.
    elected_normal_regime BOOLEAN     NOT NULL DEFAULT false,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (agency_id, owner_id, year)
);

CREATE INDEX idx_owner_annual_income_agency_year ON owner_annual_rental_income (agency_id, year);

-- ─────────────────────────────────────────────────────────────────────────────
-- 34. late_fee_policy
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE late_fee_policy (
    id                UUID          NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id         UUID          NOT NULL,
    name              TEXT          NOT NULL,
    is_default        BOOLEAN       NOT NULL DEFAULT false,
    grace_period_days SMALLINT      NOT NULL DEFAULT 5,
    fee_type          TEXT          NOT NULL CHECK (fee_type IN ('flat', 'percent')),
    flat_amount_kes   NUMERIC(12,2),
    percent_of_rent   NUMERIC(5,4),
    compounding       BOOLEAN       NOT NULL DEFAULT false,
    max_fee_kes       NUMERIC(12,2),
    created_at        TIMESTAMPTZ   NOT NULL DEFAULT now(),
    UNIQUE (agency_id, name)
);

CREATE UNIQUE INDEX idx_late_fee_one_default ON late_fee_policy (agency_id) WHERE is_default = true;
CREATE INDEX idx_late_fee_agency             ON late_fee_policy (agency_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 35. agency_integrations
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE agency_integrations (
    id            UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id     UUID        NOT NULL,
    provider_type TEXT        NOT NULL,   -- 'sms', 'email', 'payment', 'storage', 'esign'
    provider_key  TEXT        NOT NULL,   -- 'africas_talking', 'mpesa', 'sendgrid'
    is_active     BOOLEAN     NOT NULL DEFAULT true,
    -- AES-256-GCM encrypted at the application layer before INSERT
    credentials   JSONB       NOT NULL DEFAULT '{}',
    settings      JSONB       NOT NULL DEFAULT '{}',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agency_id, provider_type, provider_key)
);

CREATE INDEX idx_integrations_active ON agency_integrations (agency_id, provider_type) WHERE is_active = true;

-- ─────────────────────────────────────────────────────────────────────────────
-- 36. agency_notification_toggles
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE agency_notification_toggles (
    agency_id        UUID    NOT NULL,
    event_key        TEXT    NOT NULL,
    sms_enabled      BOOLEAN NOT NULL DEFAULT true,
    email_enabled    BOOLEAN NOT NULL DEFAULT true,
    whatsapp_enabled BOOLEAN NOT NULL DEFAULT false,
    PRIMARY KEY (agency_id, event_key)
);

-- ─────────────────────────────────────────────────────────────────────────────
-- 37. notification_templates  (MiniJinja strings stored in DB)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE notification_templates (
    id         UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id  UUID        NOT NULL,
    channel    TEXT        NOT NULL CHECK (channel IN ('sms', 'email', 'whatsapp')),
    event_key  TEXT        NOT NULL,
    locale     TEXT        NOT NULL DEFAULT 'en',
    subject    TEXT,
    body       TEXT        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agency_id, channel, event_key, locale)
);

CREATE INDEX idx_notif_templates_agency ON notification_templates (agency_id, channel, event_key);

-- ─────────────────────────────────────────────────────────────────────────────
-- 38. workflow_rules  (JSONLogic conditions + action arrays)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE workflow_rules (
    id           UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id    UUID        NOT NULL,
    name         TEXT        NOT NULL,
    event_type   TEXT        NOT NULL,
    is_active    BOOLEAN     NOT NULL DEFAULT true,
    offset_hours INT         NOT NULL DEFAULT 0,
    conditions   JSONB       NOT NULL DEFAULT '{}',
    actions      JSONB       NOT NULL DEFAULT '[]',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agency_id, name)
);

CREATE INDEX idx_workflow_rules_agency_event ON workflow_rules (agency_id, event_type) WHERE is_active = true;

-- ─────────────────────────────────────────────────────────────────────────────
-- 39. workflow_executions
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE workflow_executions (
    id          UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    rule_id     UUID        NOT NULL REFERENCES workflow_rules (id),
    agency_id   UUID        NOT NULL,
    entity_type TEXT        NOT NULL,
    entity_id   UUID        NOT NULL,
    status      TEXT        NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'running', 'completed', 'failed')),
    error       TEXT,
    executed_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_workflow_exec_rule   ON workflow_executions (rule_id, status);
CREATE INDEX idx_workflow_exec_entity ON workflow_executions (agency_id, entity_type, entity_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 40. report_configs
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE report_configs (
    id                UUID     NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id         UUID     NOT NULL,
    report_type       TEXT     NOT NULL,
    name              TEXT     NOT NULL,
    columns           JSONB    NOT NULL DEFAULT '[]',
    filters           JSONB    NOT NULL DEFAULT '{}',
    highlight_rules   JSONB    NOT NULL DEFAULT '[]',
    fiscal_year_start SMALLINT NOT NULL DEFAULT 1,
    coa_mapping       JSONB    NOT NULL DEFAULT '{}',
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agency_id, report_type, name)
);

CREATE INDEX idx_report_configs_agency ON report_configs (agency_id, report_type);

-- ─────────────────────────────────────────────────────────────────────────────
-- 41. scheduled_reports
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE scheduled_reports (
    id            UUID     NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    config_id     UUID     NOT NULL REFERENCES report_configs (id) ON DELETE CASCADE,
    agency_id     UUID     NOT NULL,
    frequency     TEXT     NOT NULL CHECK (frequency IN ('daily', 'weekly', 'monthly')),
    day_of_week   SMALLINT,
    day_of_month  SMALLINT,
    recipients    JSONB    NOT NULL DEFAULT '[]',
    export_format TEXT     NOT NULL DEFAULT 'pdf'
        CHECK (export_format IN ('pdf', 'xlsx', 'csv')),
    last_sent_at  TIMESTAMPTZ,
    is_active     BOOLEAN  NOT NULL DEFAULT true,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_scheduled_reports_agency ON scheduled_reports (agency_id) WHERE is_active = true;

-- ─────────────────────────────────────────────────────────────────────────────
-- 42. onboarding_checklists
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE onboarding_checklists (
    id         UUID NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    plan_key   TEXT,    -- NULL = applies to all plans
    name       TEXT NOT NULL,
    steps      JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ─────────────────────────────────────────────────────────────────────────────
-- 43. agency_onboarding_progress
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE agency_onboarding_progress (
    agency_id       UUID NOT NULL PRIMARY KEY,
    checklist_id    UUID NOT NULL REFERENCES onboarding_checklists (id),
    completed_steps JSONB NOT NULL DEFAULT '[]',
    dismissed_at    TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ─────────────────────────────────────────────────────────────────────────────
-- 44. data_retention_policy
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE data_retention_policy (
    agency_id              UUID     NOT NULL PRIMARY KEY,
    financial_retain_years SMALLINT NOT NULL DEFAULT 7,
    document_retain_years  SMALLINT NOT NULL DEFAULT 7,
    pii_erasure_enabled    BOOLEAN  NOT NULL DEFAULT true,
    cold_storage_bucket    TEXT,
    archival_trigger       TEXT     NOT NULL DEFAULT 'agreement_expiry'
        CHECK (archival_trigger IN ('agreement_expiry', 'calendar_year', 'manual')),
    archive_offset_months  SMALLINT NOT NULL DEFAULT 6,
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ─────────────────────────────────────────────────────────────────────────────
-- 45. archival_log
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE archival_log (
    id           UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id    UUID        NOT NULL,
    entity_type  TEXT        NOT NULL,
    entity_id    UUID        NOT NULL,
    action       TEXT        NOT NULL CHECK (action IN ('archived', 'anonymised', 'deleted')),
    archive_key  TEXT,
    performed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    performed_by UUID
);

CREATE INDEX idx_archival_log_agency ON archival_log (agency_id, entity_type, performed_at DESC);

-- ─────────────────────────────────────────────────────────────────────────────
-- 46. agency_feature_flags
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE agency_feature_flags (
    id         UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id  UUID        NOT NULL,
    key        TEXT        NOT NULL,
    enabled    BOOLEAN     NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agency_id, key)
);

CREATE INDEX idx_feature_flags_key ON agency_feature_flags (key) WHERE enabled = true;

-- ─────────────────────────────────────────────────────────────────────────────
-- FUNCTIONS & TRIGGERS
-- ─────────────────────────────────────────────────────────────────────────────

CREATE OR REPLACE FUNCTION tenant_set_updated_at()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

DO $$
DECLARE
    tbl TEXT;
BEGIN
    FOREACH tbl IN ARRAY ARRAY[
        'properties', 'units', 'residents', 'agreements', 'owners',
        'payment_claims', 'work_orders', 'vendors', 'applicants',
        'disbursements', 'inspections', 'invoices', 'caretakers',
        'work_order_comments', 'documents', 'accounts',
        'tax_obligations', 'late_fee_policy',
        'agency_integrations', 'agency_feature_flags',
        'agency_onboarding_progress', 'data_retention_policy'
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

CREATE OR REPLACE FUNCTION sync_unit_status()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.status = 'active' THEN
        UPDATE units SET status = 'occupied', updated_at = now() WHERE id = NEW.unit_id;
    ELSIF NEW.status IN ('terminated', 'expired') THEN
        UPDATE units SET status = 'vacant', updated_at = now() WHERE id = NEW.unit_id;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_agreement_unit_status
    AFTER INSERT OR UPDATE OF status ON agreements
    FOR EACH ROW EXECUTE FUNCTION sync_unit_status();

-- ─────────────────────────────────────────────────────────────────────────────
-- ANALYTICS VIEWS
-- ─────────────────────────────────────────────────────────────────────────────

CREATE OR REPLACE VIEW v_monthly_occupancy AS
WITH month_series AS (
    SELECT p.id AS property_id, p.name AS property_name, p.city,
           date_trunc('month', a.start_date)::date AS month
    FROM properties p
    JOIN agreements a ON a.property_id = p.id
    WHERE a.status IN ('active', 'expired', 'terminated')
    GROUP BY p.id, p.name, p.city, date_trunc('month', a.start_date)
)
SELECT
    ms.property_id, ms.property_name, ms.city, ms.month,
    COUNT(u.id) AS total_units,
    COUNT(u.id) FILTER (WHERE u.status = 'occupied') AS occupied_units,
    COUNT(u.id) FILTER (WHERE u.status = 'vacant')   AS vacant_units,
    CASE WHEN COUNT(u.id) = 0 THEN 0::numeric
         ELSE ROUND(COUNT(u.id) FILTER (WHERE u.status = 'occupied')::numeric
                    / COUNT(u.id)::numeric * 100, 2)
    END AS occupancy_rate_pct
FROM month_series ms
JOIN units u ON u.property_id = ms.property_id
GROUP BY ms.property_id, ms.property_name, ms.city, ms.month;

CREATE OR REPLACE VIEW v_monthly_revenue AS
SELECT
    p.id AS property_id, p.name AS property_name,
    date_trunc('month', le.posted_at)::date AS month,
    COALESCE(SUM(le.amount_kes) FILTER (WHERE le.entry_type IN (
        'rent', 'deposit', 'hoa_dues', 'cam_charge', 'utility',
        'maintenance_charge', 'late_fee', 'legal_fee', 'penalty'
    )), 0) AS charged_kes,
    COALESCE(SUM(le.amount_kes) FILTER (WHERE le.entry_type IN (
        'payment_mpesa', 'payment_bank', 'payment_cash'
    )), 0) AS collected_kes,
    COALESCE(SUM(le.amount_kes) FILTER (WHERE le.entry_type = 'late_fee'), 0) AS late_fees_kes
FROM properties p
JOIN units u         ON u.property_id = p.id
JOIN agreements a    ON a.unit_id = u.id
JOIN ledger_entries le ON le.agreement_id = a.id
GROUP BY p.id, p.name, date_trunc('month', le.posted_at);

CREATE OR REPLACE VIEW v_monthly_maintenance_cost AS
SELECT
    wo.property_id,
    date_trunc('month', wo.created_at)::date AS month,
    COUNT(*)                                  AS work_order_count,
    COALESCE(SUM(wo.actual_cost_kes), 0)     AS actual_cost_kes,
    COALESCE(SUM(wo.estimated_cost_kes), 0)  AS estimated_cost_kes,
    wo.category::text                         AS category,
    COALESCE(AVG(EXTRACT(EPOCH FROM (wo.completed_at - wo.created_at)) / 86400)
             FILTER (WHERE wo.completed_at IS NOT NULL), 0) AS avg_resolution_days
FROM work_orders wo
WHERE wo.status IN ('completed', 'cancelled', 'in_progress', 'open')
GROUP BY wo.property_id, date_trunc('month', wo.created_at), wo.category;

CREATE OR REPLACE VIEW v_vendor_performance AS
SELECT
    v.id AS vendor_id, v.name AS vendor_name, v.phone, v.email, v.speciality,
    COUNT(wo.id)                                   AS total_jobs,
    COUNT(wo.id) FILTER (WHERE wo.status = 'completed') AS completed_jobs,
    COALESCE(AVG(EXTRACT(EPOCH FROM (wo.completed_at - wo.created_at)) / 86400)
             FILTER (WHERE wo.completed_at IS NOT NULL), 0) AS avg_resolution_days,
    COALESCE(AVG(wo.actual_cost_kes)
             FILTER (WHERE wo.actual_cost_kes IS NOT NULL AND wo.actual_cost_kes > 0), 0) AS avg_cost_kes,
    MAX(wo.created_at)  AS last_job_at,
    wo.category::text   AS category
FROM vendors v
LEFT JOIN work_orders wo ON wo.vendor_id = v.id
WHERE v.status = 'active'
GROUP BY v.id, v.name, v.phone, v.email, v.speciality, wo.category;

CREATE OR REPLACE VIEW v_agreement_payment_history AS
SELECT
    a.id AS agreement_id, a.resident_id, a.unit_id, a.property_id,
    a.rent_amount_kes, a.end_date,
    COALESCE(SUM(le.amount_kes) FILTER (WHERE le.entry_type IN (
        'rent', 'deposit', 'hoa_dues', 'cam_charge', 'utility',
        'maintenance_charge', 'late_fee', 'legal_fee', 'penalty'
    )), 0)
    - COALESCE(SUM(le.amount_kes) FILTER (WHERE le.entry_type IN (
        'payment_mpesa', 'payment_bank', 'payment_cash',
        'deposit_refund', 'credit_note', 'waiver'
    )), 0) AS outstanding_kes,
    COUNT(lfc.id) FILTER (WHERE lfc.charged_at > now() - INTERVAL '6 months') AS late_payments_6m,
    MAX(le.posted_at) FILTER (WHERE le.entry_type IN (
        'payment_mpesa', 'payment_bank', 'payment_cash'
    )) AS last_payment_at
FROM agreements a
LEFT JOIN ledger_entries le  ON le.agreement_id = a.id
LEFT JOIN late_fee_charges lfc ON lfc.agreement_id = a.id
WHERE a.status = 'active'
GROUP BY a.id, a.resident_id, a.unit_id, a.property_id, a.rent_amount_kes, a.end_date;
