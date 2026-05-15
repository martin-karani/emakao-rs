
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
    'rejected',
);

CREATE TYPE ledger_entry_type AS ENUM (
    'charge_rent',
    'charge_deposit',
    'charge_hoa',
    'charge_utility',
    'charge_late_fee',
    'charge_other',
    'payment_mpesa',
    'payment_bank',
    'payment_cash',
    'payment_other',
    'credit',
    'waiver',
    'refund'
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

-- ── PROPERTIES 
CREATE TABLE properties (
    id               UUID          NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id        UUID          NOT NULL,
    name             TEXT          NOT NULL CHECK (length(trim(name)) > 0),
    address          TEXT          NOT NULL CHECK (length(trim(address)) > 0),
    city             TEXT          NOT NULL DEFAULT '',
    country_code     CHAR(2)       NOT NULL DEFAULT 'KE',
    property_type    property_type NOT NULL,
    config           JSONB         NOT NULL DEFAULT '{}',

    -- ── Work order code generation ──────────────────────────────────────────
    -- Short uppercase prefix, unique within this agency schema.
    -- Auto-generated from property name; can be overridden on creation.
    -- Examples: MG, MGRD, PARKL, SA2
    work_order_prefix VARCHAR(8)   NOT NULL DEFAULT '',
    CHECK (work_order_prefix ~ '^[A-Z][A-Z0-9]{1,7}$'),

    -- Monotonic counter — incremented atomically each time a work order is created.
    work_order_seq   INT           NOT NULL DEFAULT 0 CHECK (work_order_seq >= 0),

    created_by       UUID          NOT NULL,
    created_at       TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ   NOT NULL DEFAULT now(),

    -- Prefix must be unique within the agency schema
    UNIQUE (work_order_prefix)
);

CREATE INDEX idx_properties_agency     ON properties (agency_id);
CREATE INDEX idx_properties_type       ON properties (agency_id, property_type);
CREATE INDEX idx_properties_created_by ON properties (created_by);

-- ── UNITS ─────────────────────────────────────────────────────────────────────

CREATE TABLE units (
    id              UUID           NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    property_id     UUID           NOT NULL REFERENCES properties (id) ON DELETE CASCADE,
    -- Supports parent-unit / sub-unit hierarchy (NULL = top-level)
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

-- ── RESIDENTS ─────────────────────────────────────────────────────────────────

CREATE TABLE residents (
    id            UUID          NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    -- Cross-DB reference to platform users.id; no FK constraint
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

-- ── AGREEMENTS ────────────────────────────────────────────────────────────────

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

-- Enforce at most one active agreement per unit
CREATE UNIQUE INDEX idx_agreements_active_unit
    ON agreements (unit_id)
    WHERE status = 'active';

-- ── OWNERS ────────────────────────────────────────────────────────────────────

CREATE TABLE owners (
    id            UUID          NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    -- Cross-DB reference to platform users.id
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

-- ── PROPERTY OWNERS (ownership allocation) ────────────────────────────────────

CREATE TABLE property_owners (
    property_id       UUID           NOT NULL REFERENCES properties (id) ON DELETE CASCADE,
    owner_id          UUID           NOT NULL REFERENCES owners     (id) ON DELETE CASCADE,
    ownership_percent NUMERIC(5,2)   NOT NULL DEFAULT 100
                                     CHECK (ownership_percent > 0 AND ownership_percent <= 100),
    PRIMARY KEY (property_id, owner_id)
);

CREATE INDEX idx_property_owners_owner ON property_owners (owner_id);

-- ── PAYMENT CLAIMS ────────────────────────────────────────────────────────────

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
CREATE INDEX idx_payment_claims_agreement ON payment_claims (agreement_id)
    WHERE agreement_id IS NOT NULL;
CREATE INDEX idx_payment_claims_resident  ON payment_claims (resident_id)
    WHERE resident_id IS NOT NULL;
CREATE INDEX idx_payment_claims_submitted_by ON payment_claims (submitted_by);

-- ── LEDGER ENTRIES ────────────────────────────────────────────────────────────

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

CREATE INDEX idx_ledger_agreement  ON ledger_entries (agreement_id, posted_at DESC)
    WHERE agreement_id IS NOT NULL;
CREATE INDEX idx_ledger_unit       ON ledger_entries (unit_id)       WHERE unit_id IS NOT NULL;
CREATE INDEX idx_ledger_resident   ON ledger_entries (resident_id)   WHERE resident_id IS NOT NULL;
CREATE INDEX idx_ledger_type       ON ledger_entries (entry_type, posted_at DESC);
CREATE INDEX idx_ledger_mpesa      ON ledger_entries (mpesa_receipt) WHERE mpesa_receipt IS NOT NULL;
CREATE INDEX idx_ledger_unreconciled ON ledger_entries (posted_at)   WHERE is_reconciled = false;

-- ── VENDORS ───────────────────────────────────────────────────────────────────

CREATE TABLE vendors (
    id            UUID          NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    -- Cross-DB agency reference
    agency_id     UUID          NOT NULL,
    -- Cross-DB user reference (optional — vendor portal user)
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


-- ── CARETAKERS ────────────────────────────────────────────────────────────────
-- A caretaker is a on-site property manager distinct from agency staff.
-- They can create work orders and comment on them.

CREATE TABLE caretakers (
    id           UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    property_id  UUID        NOT NULL REFERENCES properties (id) ON DELETE CASCADE,
    -- Cross-DB reference to platform users.id (nullable = not yet invited)
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

-- ── WORK ORDERS ─────────────────────
CREATE TABLE work_orders (
    id                   UUID                     NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    property_id          UUID                     NOT NULL REFERENCES properties (id) ON DELETE RESTRICT,
    unit_id              UUID                     REFERENCES units      (id) ON DELETE SET NULL,
    vendor_id            UUID                     REFERENCES vendors    (id) ON DELETE SET NULL,

    work_order_number    INT                      NOT NULL,
    -- Full code, e.g. "MGRD-0042". Immutable after creation.
    code                 TEXT                     NOT NULL,
    UNIQUE (code),
    CHECK  (code ~ '^[A-Z][A-Z0-9]{1,7}-[0-9]+$'),

    -- ── Core fields ───────────────────────────────────────────────────────────
    title                TEXT                     NOT NULL CHECK (length(trim(title)) > 0),
    description          TEXT,
    category             work_order_category      NOT NULL DEFAULT 'general',
    status               work_order_status        NOT NULL DEFAULT 'open',
    priority             work_order_priority      NOT NULL DEFAULT 'medium',

    -- ── Reporter ─────────────────────────────────────────────────────────────
    reported_by          UUID                     NOT NULL,
    reporter_type        work_order_reporter_type NOT NULL DEFAULT 'staff',
    reporter_resident_id UUID                     REFERENCES residents  (id) ON DELETE SET NULL,
    reporter_caretaker_id UUID                    REFERENCES caretakers (id) ON DELETE SET NULL,

    -- ── Assignment ───────────────────────────────────────────────────────────
    assigned_to          UUID,
    assigned_caretaker_id UUID                    REFERENCES caretakers (id) ON DELETE SET NULL,

    -- ── Scheduling & cost ────────────────────────────────────────────────────
    due_date             DATE,
    scheduled_at         TIMESTAMPTZ,
    started_at           TIMESTAMPTZ,
    completed_at         TIMESTAMPTZ,
    estimated_cost_kes   NUMERIC(14,2)            CHECK (estimated_cost_kes >= 0),
    actual_cost_kes      NUMERIC(14,2)            CHECK (actual_cost_kes >= 0),

    -- ── Visibility & notes ────────────────────────────────────────────────────
    is_tenant_visible    BOOLEAN                  NOT NULL DEFAULT true,
    internal_notes       TEXT,
    attachments          JSONB                    NOT NULL DEFAULT '[]',

    created_at           TIMESTAMPTZ              NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ              NOT NULL DEFAULT now()
);

CREATE INDEX idx_work_orders_property      ON work_orders (property_id, status);
CREATE INDEX idx_work_orders_code          ON work_orders (code);          -- fast lookup by code
CREATE INDEX idx_work_orders_unit          ON work_orders (unit_id)        WHERE unit_id IS NOT NULL;
CREATE INDEX idx_work_orders_vendor        ON work_orders (vendor_id)      WHERE vendor_id IS NOT NULL;
CREATE INDEX idx_work_orders_assigned      ON work_orders (assigned_to)    WHERE assigned_to IS NOT NULL;
CREATE INDEX idx_work_orders_caretaker     ON work_orders (assigned_caretaker_id)
    WHERE assigned_caretaker_id IS NOT NULL;
CREATE INDEX idx_work_orders_reporter_res  ON work_orders (reporter_resident_id)
    WHERE reporter_resident_id IS NOT NULL;
CREATE INDEX idx_work_orders_due           ON work_orders (due_date)
    WHERE status NOT IN ('completed', 'cancelled') AND due_date IS NOT NULL;
CREATE INDEX idx_work_orders_open_priority
    ON work_orders (priority, created_at DESC)
    WHERE status NOT IN ('completed', 'cancelled');


-- ── WORK ORDER COMMENTS ───────────────────────────────────────────────────────
-- Threaded: parent_comment_id = NULL means top-level; non-null = reply.
-- Any actor type (staff, resident, caretaker, vendor, owner) may comment.

CREATE TABLE work_order_comments (
    id               UUID                            NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    work_order_id    UUID                            NOT NULL REFERENCES work_orders (id) ON DELETE CASCADE,
    -- NULL = top-level comment; non-null = reply to another comment
    parent_comment_id UUID                           REFERENCES work_order_comments (id) ON DELETE CASCADE,

    -- Who wrote this
    author_id        UUID                            NOT NULL,  -- cross-DB user_id
    author_type      work_order_comment_author_type  NOT NULL DEFAULT 'staff',
    -- Optional back-refs for resident / caretaker lookups
    author_resident_id  UUID                         REFERENCES residents  (id) ON DELETE SET NULL,
    author_caretaker_id UUID                         REFERENCES caretakers (id) ON DELETE SET NULL,

    body             TEXT                            NOT NULL CHECK (length(trim(body)) > 0),

    -- Whether only staff/caretakers can see this comment
    is_internal      BOOLEAN                         NOT NULL DEFAULT false,

    -- Images / docs attached to this comment
    -- Same JSONB schema as work_orders.attachments
    attachments      JSONB                           NOT NULL DEFAULT '[]',

    is_edited        BOOLEAN                         NOT NULL DEFAULT false,
    created_at       TIMESTAMPTZ                     NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ                     NOT NULL DEFAULT now()
);

CREATE INDEX idx_wo_comments_work_order  ON work_order_comments (work_order_id, created_at ASC);
CREATE INDEX idx_wo_comments_parent      ON work_order_comments (parent_comment_id)
    WHERE parent_comment_id IS NOT NULL;
CREATE INDEX idx_wo_comments_author      ON work_order_comments (author_id);

-- ── WORK ORDER ACTIVITY LOG ───────────────────────────────────────────────────
-- Immutable audit trail. Written by the application on every state change.
-- event_type examples: status_changed, comment_added, vendor_assigned,
--   priority_changed, caretaker_assigned, attachment_added, cost_updated.

CREATE TABLE work_order_activity (
    id            UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    work_order_id UUID        NOT NULL REFERENCES work_orders (id) ON DELETE CASCADE,
    actor_id      UUID        NOT NULL,
    actor_type    work_order_comment_author_type NOT NULL DEFAULT 'staff',
    event_type    TEXT        NOT NULL CHECK (length(trim(event_type)) > 0),
    -- Flexible payload, e.g. {"from":"open","to":"in_progress"} for status_changed
    payload       JSONB       NOT NULL DEFAULT '{}',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_wo_activity_work_order ON work_order_activity (work_order_id, created_at ASC);

-- ── UTILITY METERS ────────────────────────────────────────────────────────────

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

-- ── METER READINGS ────────────────────────────────────────────────────────────

CREATE TABLE meter_readings (
    id            UUID           NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    meter_id      UUID           NOT NULL REFERENCES utility_meters (id) ON DELETE CASCADE,
    reading_value NUMERIC(14,4)  NOT NULL CHECK (reading_value >= 0),
    read_at       TIMESTAMPTZ    NOT NULL DEFAULT now(),
    recorded_by   UUID           NOT NULL
);

CREATE INDEX idx_meter_readings_meter ON meter_readings (meter_id, read_at DESC);

-- ── UTILITY BILLS ─────────────────────────────────────────────────────────────

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

-- ── APPLICANTS ────────────────────────────────────────────────────────────────

CREATE TABLE applicants (
    id                 UUID               NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    -- Cross-DB agency reference
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

-- ── DISBURSEMENTS ─────────────────────────────────────────────────────────────

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

-- ── INSPECTIONS ───────────────────────────────────────────────────────────────

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

-- ── INVOICES ──────────────────────────────────────────────────────────────────

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
    created_by     UUID           NOT NULL,
    created_at     TIMESTAMPTZ    NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ    NOT NULL DEFAULT now()
);

CREATE INDEX idx_invoices_property  ON invoices (property_id, status);
CREATE INDEX idx_invoices_agreement ON invoices (agreement_id) WHERE agreement_id IS NOT NULL;
CREATE INDEX idx_invoices_resident  ON invoices (resident_id)  WHERE resident_id IS NOT NULL;
CREATE INDEX idx_invoices_due       ON invoices (due_date)     WHERE status NOT IN ('paid','void');

-- ── CONVERSATIONS ─────────────────────────────────────────────────────────────

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

-- ── MESSAGES ─────────────────────────────────────────────────────────────────

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

-- ── UPDATED_AT TRIGGER (applied to all mutable tables) ───────────────────────

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
        'properties',
        'units',
        'residents',
        'agreements',
        'owners',
        'payment_claims',
        'work_orders',
        'vendors',
        'applicants',
        'disbursements',
        'inspections',
        'invoices'
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

-- ── TRIGGER: sync unit status from agreement state machine ────────────────────

CREATE OR REPLACE FUNCTION sync_unit_status()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.status = 'active' THEN
        UPDATE units
        SET    status     = 'occupied',
               updated_at = now()
        WHERE  id = NEW.unit_id;

    ELSIF NEW.status IN ('terminated', 'expired') THEN
        UPDATE units
        SET    status     = 'vacant',
               updated_at = now()
        WHERE  id = NEW.unit_id;
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_agreement_unit_status
    AFTER INSERT OR UPDATE OF status ON agreements
    FOR EACH ROW EXECUTE FUNCTION sync_unit_status();