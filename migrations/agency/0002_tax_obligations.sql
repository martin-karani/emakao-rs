-- ─────────────────────────────────────────────────────────────────────────────
-- migrations/agency/0002_tax_obligations.sql
--
-- Adds two tables required for Kenya tax compliance:
--   1. tax_obligations   — one row per owner × property × period × tax head
--   2. owner_annual_rental_income — rolling annual tracker for MRI regime check
--
-- Run after: migrations/agency/0001_initial.sql
-- ─────────────────────────────────────────────────────────────────────────────

-- ── 1. tax_obligations ────────────────────────────────────────────────────────

CREATE TYPE tax_obligation_type AS ENUM ('mri', 'vat', 'wht');
CREATE TYPE tax_obligation_status AS ENUM ('pending', 'filed', 'paid', 'overdue', 'nil_filed');

CREATE TABLE IF NOT EXISTS tax_obligations (
    id                  UUID         NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id           UUID         NOT NULL,
    owner_id            UUID         NOT NULL,
    property_id         UUID         NOT NULL,
    agreement_id        UUID,

    -- Tax classification
    obligation_type     tax_obligation_type   NOT NULL,
    -- First day of the calendar month (e.g. 2025-04-01 = April 2025)
    tax_period          DATE         NOT NULL,
    -- Gross amount on which tax is computed:
    --   MRI  → gross rent for the period
    --   VAT  → management fee (exclusive of VAT)
    --   WHT  → management fee (gross, before deduction)
    gross_amount_kes    NUMERIC(18,2) NOT NULL CHECK (gross_amount_kes >= 0),
    tax_kes             NUMERIC(18,2) NOT NULL CHECK (tax_kes >= 0),
    tax_rate            NUMERIC(8,4)  NOT NULL,          -- e.g. 0.0750, 0.1600, 0.0500
    due_date            DATE         NOT NULL,

    -- Lifecycle
    status              tax_obligation_status NOT NULL DEFAULT 'pending',

    -- KRA references (populated after filing / payment)
    kra_prn             TEXT,          -- Payment Registration Number
    kra_ack_number      TEXT,          -- eRITS / iTax acknowledgement number

    filed_at            TIMESTAMPTZ,
    remitted_at         TIMESTAMPTZ,

    -- Audit
    created_at          TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    -- Prevent duplicate obligations for the same cell
    UNIQUE (agency_id, owner_id, property_id, tax_period, obligation_type)
);

CREATE INDEX idx_tax_obligations_agency_period
    ON tax_obligations (agency_id, tax_period);

CREATE INDEX idx_tax_obligations_owner
    ON tax_obligations (owner_id, tax_period);

CREATE INDEX idx_tax_obligations_status
    ON tax_obligations (agency_id, status)
    WHERE status IN ('pending', 'overdue');

CREATE INDEX idx_tax_obligations_due_date
    ON tax_obligations (due_date)
    WHERE status NOT IN ('paid', 'nil_filed');

-- Auto-update updated_at
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;

CREATE TRIGGER tax_obligations_updated_at
    BEFORE UPDATE ON tax_obligations
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();


-- ── 2. owner_annual_rental_income ─────────────────────────────────────────────
--
-- Updated monthly by the tax compliance worker.
-- Used to determine which MRI regime applies for each period:
--   < 288,000    → exempt
--   288,001–15M  → MRI at 7.5 % (final tax)
--   > 15M        → normal income tax

CREATE TYPE mri_regime AS ENUM ('exempt', 'mri', 'normal_income_tax', 'elected_normal');

CREATE TABLE IF NOT EXISTS owner_annual_rental_income (
    id                      UUID         NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id               UUID         NOT NULL,
    owner_id                UUID         NOT NULL,
    year                    SMALLINT     NOT NULL,
    total_gross_rent_kes    NUMERIC(18,2) NOT NULL DEFAULT 0,
    mri_regime              mri_regime   NOT NULL DEFAULT 'exempt',
    -- Set to TRUE when the owner has formally elected the normal income-tax
    -- regime in writing (Form ITR-2 election).
    elected_normal_regime   BOOLEAN      NOT NULL DEFAULT FALSE,
    updated_at              TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    UNIQUE (agency_id, owner_id, year)
);

CREATE INDEX idx_owner_annual_income_agency_year
    ON owner_annual_rental_income (agency_id, year);


-- ── 3. invoice — add new columns ─────────────────────────────────────────────
--
-- Migrates the old single `tax_kes` to the new per-type breakdown.
-- The old column is kept as mri_kes for backwards compatibility but is
-- populated by the TaxBreakdown computed value going forward.

ALTER TABLE invoices
    ADD COLUMN IF NOT EXISTS owner_id       UUID,
    ADD COLUMN IF NOT EXISTS mri_kes        NUMERIC(18,2) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS vat_kes        NUMERIC(18,2) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS wht_kes        NUMERIC(18,2) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS net_payable_kes NUMERIC(18,2),
    ADD COLUMN IF NOT EXISTS mri_regime     mri_regime,
    ADD COLUMN IF NOT EXISTS tax_period     DATE,
    ADD COLUMN IF NOT EXISTS tax_due_date   DATE,
    -- eTIMS reference — set after fiscal device submission
    ADD COLUMN IF NOT EXISTS etims_cu_invoice_number TEXT,
    ADD COLUMN IF NOT EXISTS etims_qr_code           TEXT,
    ADD COLUMN IF NOT EXISTS etims_accepted_at       TIMESTAMPTZ;

-- Migrate existing rows: populate new breakdown from the old `tax_kes`
-- (we treat old tax_kes as VAT for conservative migration)
UPDATE invoices
SET    vat_kes = tax_kes
WHERE  vat_kes = 0
  AND  tax_kes > 0;