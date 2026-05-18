
-- ─────────────────────────────────────────────────────────────────────────────
-- HELPERS
-- ─────────────────────────────────────────────────────────────────────────────

-- UUID v7 implementation for PostgreSQL
CREATE OR REPLACE FUNCTION uuidv7()
RETURNS uuid AS $$
DECLARE
    v_time timestamp with time zone:= clock_timestamp();
    v_unix_t bigint;
    v_rand_a int;
    v_rand_b bigint;
BEGIN
    v_unix_t:= (EXTRACT(EPOCH FROM v_time) * 1000)::bigint;
    v_rand_a:= floor(random() * 4096)::int;
    v_rand_b:= (random() * 4611686018427387904)::bigint; -- 2^62

    RETURN encode(
        set_byte(
            set_byte(
                decode(
                    lpad(to_hex(v_unix_t), 12, '0') ||
                    lpad(to_hex(v_rand_a), 4, '0') ||
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
    'rejected'          -- trailing comma removed
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

-- ─────────────────────────────────────────────────────────────────────────────
-- TABLES
-- ─────────────────────────────────────────────────────────────────────────────

-- PROPERTIES
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

-- UNITS
CREATE TABLE units (
    id              UUID           NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    property_id     UUID           NOT NULL REFERENCES properties (id) ON DELETE CASCADE,
    parent_unit_id  UUID           REFERENCES units (id) ON DELETE SET NULL,
    unit_number     TEXT           NOT NULL CHECK (length(trim(unit_number)) > 0),
    floor           INTEGER        CHECK (floor >= -10),
    size_sqm        NUMERIC(10,2)  CHECK (size_sqm > 0),
    bedrooms        SMALLINT       CHECK (bedrooms >= 0),
    bathrooms       SMALLINT       CHECK (bathrooms >= 0),
    rent_amount_kes NUMERIC(14,2)  NOT NULL DEFAULT 0 CHECK (rent_amount_kes >= 0),
    deposit_kes     NUMERIC(14,2)  NOT NULL DEFAULT 0 CHECK (deposit_kes >= 0),
    status          unit_status    NOT NULL DEFAULT 'vacant',
    description     TEXT,
    photos          JSONB          NOT NULL DEFAULT '[]',
    created_at      TIMESTAMPTZ    NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ    NOT NULL DEFAULT now(),
    UNIQUE (property_id, unit_number)
);

CREATE INDEX idx_units_property    ON units (property_id);
CREATE INDEX idx_units_status      ON units (property_id, status);
CREATE INDEX idx_units_parent      ON units (parent_unit_id) WHERE parent_unit_id IS NOT NULL;

-- RESIDENTS
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

-- AGREEMENTS
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

-- OWNERS
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

-- PROPERTY OWNERS
CREATE TABLE property_owners (
    property_id       UUID           NOT NULL REFERENCES properties (id) ON DELETE CASCADE,
    owner_id          UUID           NOT NULL REFERENCES owners     (id) ON DELETE CASCADE,
    ownership_percent NUMERIC(5,2)   NOT NULL DEFAULT 100 CHECK (ownership_percent > 0 AND ownership_percent <= 100),
    PRIMARY KEY (property_id, owner_id)
);
CREATE INDEX idx_property_owners_owner ON property_owners (owner_id);

-- PAYMENT CLAIMS
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

CREATE INDEX idx_payment_claims_property  ON payment_claims (property_id, status);
CREATE INDEX idx_payment_claims_agreement ON payment_claims (agreement_id) WHERE agreement_id IS NOT NULL;
CREATE INDEX idx_payment_claims_resident  ON payment_claims (resident_id) WHERE resident_id IS NOT NULL;
CREATE INDEX idx_payment_claims_submitted_by ON payment_claims (submitted_by);

-- LEDGER ENTRIES
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
    CONSTRAINT ledger_period_valid CHECK (period_end IS NULL OR period_start IS NULL OR period_end >= period_start)
);

CREATE INDEX idx_ledger_agreement  ON ledger_entries (agreement_id, posted_at DESC) WHERE agreement_id IS NOT NULL;
CREATE INDEX idx_ledger_unit       ON ledger_entries (unit_id)       WHERE unit_id IS NOT NULL;
CREATE INDEX idx_ledger_resident   ON ledger_entries (resident_id)   WHERE resident_id IS NOT NULL;
CREATE INDEX idx_ledger_type       ON ledger_entries (entry_type, posted_at DESC);
CREATE INDEX idx_ledger_mpesa      ON ledger_entries (mpesa_receipt) WHERE mpesa_receipt IS NOT NULL;
CREATE INDEX idx_ledger_unreconciled ON ledger_entries (posted_at)   WHERE is_reconciled = false;

-- VENDORS
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

-- CARETAKERS
CREATE TABLE caretakers (
    id           UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    property_id  UUID        NOT NULL REFERENCES properties (id) ON DELETE CASCADE,
    user_id      UUID,
    first_name   TEXT        NOT NULL CHECK (length(trim(first_name)) > 0),
    last_name    TEXT        NOT NULL CHECK (length(trim(last_name)) > 0),
    phone        TEXT,
    email        TEXT        CHECK (email = lower(email)),
    is_active    BOOLEAN     NOT NULL DEFAULT true,
    created_by   UUID        NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_caretakers_property ON caretakers (property_id, is_active);
CREATE INDEX idx_caretakers_user_id  ON caretakers (user_id) WHERE user_id IS NOT NULL;

-- WORK ORDERS
CREATE TABLE work_orders (
    id                   UUID                     NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    property_id          UUID                     NOT NULL REFERENCES properties (id) ON DELETE RESTRICT,
    unit_id              UUID                     REFERENCES units      (id) ON DELETE SET NULL,
    vendor_id            UUID                     REFERENCES vendors    (id) ON DELETE SET NULL,
    work_order_number    INT                      NOT NULL,
    code                 TEXT                     NOT NULL UNIQUE,
    CHECK  (code ~ '^[A-Z][A-Z0-9]{1,7}-[0-9]+$'),
    title                TEXT                     NOT NULL CHECK (length(trim(title)) > 0),
    description          TEXT,
    category             work_order_category      NOT NULL DEFAULT 'general',
    status               work_order_status        NOT NULL DEFAULT 'open',
    priority             work_order_priority      NOT NULL DEFAULT 'medium',
    reported_by          UUID                     NOT NULL,
    reporter_type        work_order_reporter_type NOT NULL DEFAULT 'staff',
    reporter_resident_id UUID                     REFERENCES residents  (id) ON DELETE SET NULL,
    reporter_caretaker_id UUID                    REFERENCES caretakers (id) ON DELETE SET NULL,
    assigned_to          UUID,
    assigned_caretaker_id UUID                    REFERENCES caretakers (id) ON DELETE SET NULL,
    due_date             DATE,
    scheduled_at         TIMESTAMPTZ,
    started_at           TIMESTAMPTZ,
    completed_at         TIMESTAMPTZ,
    estimated_cost_kes   NUMERIC(14,2)            CHECK (estimated_cost_kes >= 0),
    actual_cost_kes      NUMERIC(14,2)            CHECK (actual_cost_kes >= 0),
    is_tenant_visible    BOOLEAN                  NOT NULL DEFAULT true,
    internal_notes       TEXT,
    attachments          JSONB                    NOT NULL DEFAULT '[]',
    created_at           TIMESTAMPTZ              NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ              NOT NULL DEFAULT now()
);

CREATE INDEX idx_work_orders_property      ON work_orders (property_id, status);
CREATE INDEX idx_work_orders_code          ON work_orders (code);
CREATE INDEX idx_work_orders_unit          ON work_orders (unit_id) WHERE unit_id IS NOT NULL;
CREATE INDEX idx_work_orders_vendor        ON work_orders (vendor_id) WHERE vendor_id IS NOT NULL;
CREATE INDEX idx_work_orders_assigned      ON work_orders (assigned_to) WHERE assigned_to IS NOT NULL;
CREATE INDEX idx_work_orders_caretaker     ON work_orders (assigned_caretaker_id) WHERE assigned_caretaker_id IS NOT NULL;
CREATE INDEX idx_work_orders_reporter_res  ON work_orders (reporter_resident_id) WHERE reporter_resident_id IS NOT NULL;
CREATE INDEX idx_work_orders_due           ON work_orders (due_date) WHERE status NOT IN ('completed', 'cancelled') AND due_date IS NOT NULL;
CREATE INDEX idx_work_orders_open_priority ON work_orders (priority, created_at DESC) WHERE status NOT IN ('completed', 'cancelled');

-- WORK ORDER COMMENTS
CREATE TABLE work_order_comments (
    id               UUID                            NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    work_order_id    UUID                            NOT NULL REFERENCES work_orders (id) ON DELETE CASCADE,
    parent_comment_id UUID                           REFERENCES work_order_comments (id) ON DELETE CASCADE,
    author_id        UUID                            NOT NULL,
    author_type      work_order_comment_author_type  NOT NULL DEFAULT 'staff',
    author_resident_id  UUID                         REFERENCES residents  (id) ON DELETE SET NULL,
    author_caretaker_id UUID                         REFERENCES caretakers (id) ON DELETE SET NULL,
    body             TEXT                            NOT NULL CHECK (length(trim(body)) > 0),
    is_internal      BOOLEAN                         NOT NULL DEFAULT false,
    attachments      JSONB                           NOT NULL DEFAULT '[]',
    is_edited        BOOLEAN                         NOT NULL DEFAULT false,
    created_at       TIMESTAMPTZ                     NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ                     NOT NULL DEFAULT now()
);

CREATE INDEX idx_wo_comments_work_order  ON work_order_comments (work_order_id, created_at ASC);
CREATE INDEX idx_wo_comments_parent      ON work_order_comments (parent_comment_id) WHERE parent_comment_id IS NOT NULL;
CREATE INDEX idx_wo_comments_author      ON work_order_comments (author_id);

-- WORK ORDER ACTIVITY LOG
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

-- UTILITY METERS
CREATE TABLE utility_meters (
    id            UUID         NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    unit_id       UUID         NOT NULL REFERENCES units (id) ON DELETE CASCADE,
    meter_type    meter_type   NOT NULL,
    billing_mode  billing_mode NOT NULL,
    meter_number  TEXT         NOT NULL CHECK (length(trim(meter_number)) > 0),
    rate_per_unit NUMERIC(10,4) NOT NULL CHECK (rate_per_unit > 0),
    is_active     BOOLEAN      NOT NULL DEFAULT true,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT now(),
    UNIQUE (unit_id, meter_type)
);
CREATE INDEX idx_utility_meters_unit ON utility_meters (unit_id);

-- METER READINGS
CREATE TABLE meter_readings (
    id            UUID           NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    meter_id      UUID           NOT NULL REFERENCES utility_meters (id) ON DELETE CASCADE,
    reading_value NUMERIC(14,4)  NOT NULL CHECK (reading_value >= 0),
    read_at       TIMESTAMPTZ    NOT NULL DEFAULT now(),
    recorded_by   UUID           NOT NULL
);
CREATE INDEX idx_meter_readings_meter ON meter_readings (meter_id, read_at DESC);

-- UTILITY BILLS
CREATE TABLE utility_bills (
    id             UUID                NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    meter_id       UUID                NOT NULL REFERENCES utility_meters (id) ON DELETE RESTRICT,
    unit_id        UUID                NOT NULL REFERENCES units           (id) ON DELETE RESTRICT,
    units_consumed NUMERIC(14,4)       NOT NULL CHECK (units_consumed > 0),
    amount_kes     NUMERIC(14,2)       NOT NULL CHECK (amount_kes >= 0),
    status         utility_bill_status NOT NULL DEFAULT 'draft',
    billing_period DATE,
    created_at     TIMESTAMPTZ         NOT NULL DEFAULT now()
);
CREATE INDEX idx_utility_bills_unit   ON utility_bills (unit_id, status);
CREATE INDEX idx_utility_bills_meter  ON utility_bills (meter_id);

-- APPLICANTS
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

-- DISBURSEMENTS
CREATE TABLE disbursements (
    id           UUID                 NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id    UUID                 NOT NULL,
    owner_id     UUID                 NOT NULL REFERENCES owners     (id) ON DELETE RESTRICT,
    property_id  UUID                 NOT NULL REFERENCES properties (id) ON DELETE RESTRICT,
    amount_kes   NUMERIC(14,2)        NOT NULL CHECK (amount_kes > 0),
    method       disbursement_method  NOT NULL,
    reference    TEXT,
    status       disbursement_status  NOT NULL DEFAULT 'pending',
    period_start DATE                 NOT NULL,
    period_end   DATE                 NOT NULL CHECK (period_end >= period_start),
    notes        TEXT,
    created_by   UUID                 NOT NULL,
    created_at   TIMESTAMPTZ          NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ          NOT NULL DEFAULT now()
);
CREATE INDEX idx_disbursements_agency   ON disbursements (agency_id);
CREATE INDEX idx_disbursements_owner    ON disbursements (owner_id);
CREATE INDEX idx_disbursements_property ON disbursements (property_id);
CREATE INDEX idx_disbursements_status   ON disbursements (status);

-- INSPECTIONS
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

-- INVOICES
CREATE TABLE invoices (
    id             UUID           NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id      UUID           NOT NULL,
    property_id    UUID           NOT NULL REFERENCES properties (id) ON DELETE RESTRICT,
    agreement_id   UUID           REFERENCES agreements (id) ON DELETE SET NULL,
    resident_id    UUID           REFERENCES residents  (id) ON DELETE SET NULL,
    invoice_number TEXT           NOT NULL UNIQUE,
    line_items     JSONB          NOT NULL DEFAULT '[]',
    subtotal_kes   NUMERIC(14,2)  NOT NULL CHECK (subtotal_kes >= 0),
    tax_kes        NUMERIC(14,2)  NOT NULL DEFAULT 0 CHECK (tax_kes >= 0),
    total_kes      NUMERIC(14,2)  NOT NULL CHECK (total_kes >= 0),
    due_date       DATE           NOT NULL,
    status         invoice_status NOT NULL DEFAULT 'draft',
    notes          TEXT,
    voided_at      TIMESTAMPTZ,
    created_by     UUID           NOT NULL,
    created_at     TIMESTAMPTZ    NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ    NOT NULL DEFAULT now()
);
CREATE INDEX idx_invoices_property  ON invoices (property_id, status);
CREATE INDEX idx_invoices_agreement ON invoices (agreement_id) WHERE agreement_id IS NOT NULL;
CREATE INDEX idx_invoices_resident  ON invoices (resident_id)  WHERE resident_id IS NOT NULL;
CREATE INDEX idx_invoices_due       ON invoices (due_date)     WHERE status NOT IN ('paid','void');

-- CONVERSATIONS
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

-- MESSAGES
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

-- DOCUMENTS
CREATE TABLE documents (
    id              UUID           NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id       UUID           NOT NULL,
    s3_key          TEXT           NOT NULL,
    file_name       TEXT           NOT NULL,
    mime_type       TEXT           NOT NULL DEFAULT 'application/octet-stream',
    size_bytes      BIGINT         NOT NULL DEFAULT 0 CHECK (size_bytes >= 0),
    document_type   document_type  NOT NULL DEFAULT 'other',
    title           TEXT,
    notes           TEXT,
    property_id     UUID           REFERENCES properties (id) ON DELETE SET NULL,
    unit_id         UUID,
    resident_id     UUID,
    agreement_id    UUID           REFERENCES agreements (id) ON DELETE SET NULL,
    work_order_id   UUID           REFERENCES work_orders (id) ON DELETE SET NULL,
    uploaded_by     UUID           NOT NULL,
    is_deleted      BOOLEAN        NOT NULL DEFAULT false,
    deleted_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ    NOT NULL DEFAULT now()
);
CREATE INDEX idx_documents_agency     ON documents (agency_id, created_at DESC) WHERE is_deleted = false;
CREATE INDEX idx_documents_property   ON documents (property_id) WHERE property_id IS NOT NULL AND is_deleted = false;
CREATE INDEX idx_documents_unit       ON documents (unit_id) WHERE unit_id IS NOT NULL AND is_deleted = false;
CREATE INDEX idx_documents_resident   ON documents (resident_id) WHERE resident_id IS NOT NULL AND is_deleted = false;
CREATE INDEX idx_documents_agreement  ON documents (agreement_id) WHERE agreement_id IS NOT NULL AND is_deleted = false;
CREATE INDEX idx_documents_work_order ON documents (work_order_id) WHERE work_order_id IS NOT NULL AND is_deleted = false;
CREATE INDEX idx_documents_type       ON documents (agency_id, document_type) WHERE is_deleted = false;

-- BILLING SCHEDULER TABLES
CREATE TABLE rent_charges (
    id              UUID           NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agreement_id    UUID           NOT NULL REFERENCES agreements (id) ON DELETE CASCADE,
    period_start    DATE           NOT NULL,
    period_end      DATE           NOT NULL CHECK (period_end >= period_start),
    amount_kes      NUMERIC(14,2)  NOT NULL CHECK (amount_kes > 0),
    ledger_entry_id UUID,
    charged_at      TIMESTAMPTZ    NOT NULL DEFAULT now(),
    UNIQUE (agreement_id, period_start)
);
CREATE INDEX idx_rent_charges_agreement ON rent_charges (agreement_id, period_start DESC);

CREATE TABLE late_fee_charges (
    id              UUID           NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agreement_id    UUID           NOT NULL REFERENCES agreements (id) ON DELETE CASCADE,
    period_start    DATE           NOT NULL,
    amount_kes      NUMERIC(14,2)  NOT NULL CHECK (amount_kes > 0),
    ledger_entry_id UUID,
    charged_at      TIMESTAMPTZ    NOT NULL DEFAULT now(),
    UNIQUE (agreement_id, period_start)
);
CREATE INDEX idx_late_fee_charges_agreement ON late_fee_charges (agreement_id, period_start DESC);

CREATE TABLE rent_reminders_sent (
    id              UUID           NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agreement_id    UUID           NOT NULL REFERENCES agreements (id) ON DELETE CASCADE,
    period_start    DATE           NOT NULL,
    channel         TEXT           NOT NULL CHECK (channel IN ('email', 'sms')),
    sent_at         TIMESTAMPTZ    NOT NULL DEFAULT now(),
    UNIQUE (agreement_id, period_start, channel)
);
CREATE INDEX idx_rent_reminders_agreement ON rent_reminders_sent (agreement_id, period_start DESC);

CREATE TABLE late_fee_policy (
    id                  UUID           NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    grace_period_days   INT            NOT NULL DEFAULT 5 CHECK (grace_period_days >= 0),
    flat_amount_kes     NUMERIC(14,2)  CHECK (flat_amount_kes > 0),
    rate_percent        NUMERIC(5,2)   CHECK (rate_percent > 0 AND rate_percent <= 100),
    created_at          TIMESTAMPTZ    NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ    NOT NULL DEFAULT now(),
    CHECK (flat_amount_kes IS NOT NULL OR rate_percent IS NOT NULL)
);
INSERT INTO late_fee_policy (grace_period_days, rate_percent) VALUES (5, 5) ON CONFLICT DO NOTHING;

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
        'work_order_comments', 'documents', 'late_fee_policy'
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
    COUNT(u.id) FILTER (WHERE u.status = 'vacant') AS vacant_units,
    CASE WHEN COUNT(u.id) = 0 THEN 0::numeric
         ELSE ROUND(COUNT(u.id) FILTER (WHERE u.status = 'occupied')::numeric / COUNT(u.id)::numeric * 100, 2)
    END AS occupancy_rate_pct
FROM month_series ms
JOIN units u ON u.property_id = ms.property_id
GROUP BY ms.property_id, ms.property_name, ms.city, ms.month;

CREATE OR REPLACE VIEW v_monthly_revenue AS
SELECT
    p.id AS property_id, p.name AS property_name,
    date_trunc('month', le.posted_at)::date AS month,
    COALESCE(SUM(le.amount_kes) FILTER (WHERE le.entry_type IN ('rent', 'deposit', 'hoa_dues', 'cam_charge', 'utility', 'maintenance_charge', 'late_fee', 'legal_fee', 'penalty')), 0) AS charged_kes,
    COALESCE(SUM(le.amount_kes) FILTER (WHERE le.entry_type IN ('payment_mpesa', 'payment_bank', 'payment_cash')), 0) AS collected_kes,
    COALESCE(SUM(le.amount_kes) FILTER (WHERE le.entry_type = 'late_fee'), 0) AS late_fees_kes
FROM properties p
JOIN units u ON u.property_id = p.id
JOIN agreements a ON a.unit_id = u.id
JOIN ledger_entries le ON le.agreement_id = a.id
GROUP BY p.id, p.name, date_trunc('month', le.posted_at);

CREATE OR REPLACE VIEW v_monthly_maintenance_cost AS
SELECT
    wo.property_id,
    date_trunc('month', wo.created_at)::date AS month,
    COUNT(*) AS work_order_count,
    COALESCE(SUM(wo.actual_cost_kes), 0) AS actual_cost_kes,
    COALESCE(SUM(wo.estimated_cost_kes), 0) AS estimated_cost_kes,
    wo.category::text AS category,
    COALESCE(AVG(EXTRACT(EPOCH FROM (wo.completed_at - wo.created_at)) / 86400) FILTER (WHERE wo.completed_at IS NOT NULL), 0) AS avg_resolution_days
FROM work_orders wo
WHERE wo.status IN ('completed', 'cancelled', 'in_progress', 'open')
GROUP BY wo.property_id, date_trunc('month', wo.created_at), wo.category;

CREATE OR REPLACE VIEW v_vendor_performance AS
SELECT
    v.id AS vendor_id, v.name AS vendor_name, v.phone, v.email, v.speciality,
    COUNT(wo.id) AS total_jobs,
    COUNT(wo.id) FILTER (WHERE wo.status = 'completed') AS completed_jobs,
    COALESCE(AVG(EXTRACT(EPOCH FROM (wo.completed_at - wo.created_at)) / 86400) FILTER (WHERE wo.completed_at IS NOT NULL), 0) AS avg_resolution_days,
    COALESCE(AVG(wo.actual_cost_kes) FILTER (WHERE wo.actual_cost_kes IS NOT NULL AND wo.actual_cost_kes > 0), 0) AS avg_cost_kes,
    MAX(wo.created_at) AS last_job_at,
    wo.category::text AS category
FROM vendors v
LEFT JOIN work_orders wo ON wo.vendor_id = v.id
WHERE v.status = 'active'
GROUP BY v.id, v.name, v.phone, v.email, v.speciality, wo.category;

CREATE OR REPLACE VIEW v_agreement_payment_history AS
SELECT
    a.id AS agreement_id, a.resident_id, a.unit_id, a.property_id, a.rent_amount_kes, a.end_date,
    COALESCE(SUM(le.amount_kes) FILTER (WHERE le.entry_type IN ('rent', 'deposit', 'hoa_dues', 'cam_charge', 'utility', 'maintenance_charge', 'late_fee', 'legal_fee', 'penalty')), 0)
    - COALESCE(SUM(le.amount_kes) FILTER (WHERE le.entry_type IN ('payment_mpesa', 'payment_bank', 'payment_cash', 'deposit_refund', 'credit_note', 'waiver')), 0) AS outstanding_kes,
    COUNT(lfc.id) FILTER (WHERE lfc.charged_at > now() - INTERVAL '6 months') AS late_payments_6m,
    MAX(le.posted_at) FILTER (WHERE le.entry_type IN ('payment_mpesa', 'payment_bank', 'payment_cash')) AS last_payment_at
FROM agreements a
LEFT JOIN ledger_entries le ON le.agreement_id = a.id
LEFT JOIN late_fee_charges lfc ON lfc.agreement_id = a.id
WHERE a.status = 'active'
GROUP BY a.id, a.resident_id, a.unit_id, a.property_id, a.rent_amount_kes, a.end_date;

-- ── Double-entry accounting (from 0002, 0003) ────────────────────────────────

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

CREATE TABLE accounts (
    id           UUID              NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id    UUID              NOT NULL,
    code         TEXT              NOT NULL,           -- e.g. "1010", "4001"
    name         TEXT              NOT NULL,
    account_type account_category  NOT NULL,
    balance      NUMERIC(18, 2)    NOT NULL DEFAULT 0, -- running balance, updated on post
    is_system    BOOLEAN           NOT NULL DEFAULT false, -- system accounts cannot be deleted
    vat_applicable BOOLEAN         NOT NULL DEFAULT false,
    vat_rate       NUMERIC(5, 4)            DEFAULT 0.1600,
    created_at   TIMESTAMPTZ       NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ       NOT NULL DEFAULT now(),
    UNIQUE (agency_id, code)
);

CREATE INDEX idx_accounts_agency ON accounts (agency_id, account_type);

COMMENT ON COLUMN accounts.vat_applicable IS
    'True for accounts that generate output VAT (revenue) or input VAT (expense) entries.';
COMMENT ON COLUMN accounts.vat_rate IS
    'Fractional VAT rate, e.g. 0.1600 = 16 %. NULL when vat_applicable = false.';

CREATE TABLE journal_entries (
    id          UUID                  NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id   UUID                  NOT NULL,
    reference   TEXT                  NOT NULL,           -- human-readable ref, e.g. "JE-2025-001"
    description TEXT,
    status      journal_entry_status  NOT NULL DEFAULT 'draft',
    posted_by   UUID                  NOT NULL,
    posted_at   TIMESTAMPTZ           NOT NULL DEFAULT now(),
    created_at  TIMESTAMPTZ           NOT NULL DEFAULT now()
);

CREATE INDEX idx_journal_entries_agency  ON journal_entries (agency_id, posted_at DESC);
CREATE INDEX idx_journal_entries_status  ON journal_entries (agency_id, status);

CREATE TABLE journal_lines (
    id               UUID           NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    journal_entry_id UUID           NOT NULL REFERENCES journal_entries (id) ON DELETE CASCADE,
    account_id       UUID           NOT NULL REFERENCES accounts (id),
    debit_kes        NUMERIC(18, 2) NOT NULL DEFAULT 0 CHECK (debit_kes  >= 0),
    credit_kes       NUMERIC(18, 2) NOT NULL DEFAULT 0 CHECK (credit_kes >= 0),
    description      TEXT,
    created_at       TIMESTAMPTZ    NOT NULL DEFAULT now(),
    -- Each line is either a pure debit or a pure credit, never both.
    CHECK (
        (debit_kes > 0 AND credit_kes = 0) OR
        (credit_kes > 0 AND debit_kes = 0)
    )
);

CREATE INDEX idx_journal_lines_entry   ON journal_lines (journal_entry_id);
CREATE INDEX idx_journal_lines_account ON journal_lines (account_id);

CREATE TRIGGER trg_accounts_updated_at
    BEFORE UPDATE ON accounts
    FOR EACH ROW EXECUTE FUNCTION tenant_set_updated_at();

-- ── Bank reconciliation (from 0003) ──────────────────────────────────────────

CREATE TABLE bank_statements (
    id              UUID           NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id       UUID           NOT NULL,
    bank_name       TEXT           NOT NULL,
    account_number  TEXT           NOT NULL,
    statement_date  DATE           NOT NULL,
    opening_balance NUMERIC(18, 2) NOT NULL,
    closing_balance NUMERIC(18, 2) NOT NULL,
    created_by      UUID           NOT NULL,
    created_at      TIMESTAMPTZ    NOT NULL DEFAULT now()
);

CREATE INDEX idx_bank_statements_agency ON bank_statements (agency_id, statement_date DESC);

CREATE TABLE bank_statement_lines (
    id               UUID           NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    statement_id     UUID           NOT NULL REFERENCES bank_statements (id) ON DELETE CASCADE,
    value_date       DATE           NOT NULL,
    description      TEXT           NOT NULL,
    -- Positive = credit / money in.  Negative = debit / money out.
    amount           NUMERIC(18, 2) NOT NULL,
    reference        TEXT,
    -- Set when this line is matched to a posted journal entry.
    matched_entry_id UUID           REFERENCES journal_entries (id) ON DELETE SET NULL,
    created_at       TIMESTAMPTZ    NOT NULL DEFAULT now()
);

CREATE INDEX idx_bsl_statement    ON bank_statement_lines (statement_id);
CREATE INDEX idx_bsl_matched      ON bank_statement_lines (matched_entry_id)
    WHERE matched_entry_id IS NOT NULL;
CREATE INDEX idx_bsl_unreconciled ON bank_statement_lines (statement_id)
    WHERE matched_entry_id IS NULL;
