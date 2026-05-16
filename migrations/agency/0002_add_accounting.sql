-- migrations/agency/0002_add_accounting.sql
--
-- Double-entry accounting tables.
-- Gated by the `acct_double_entry` feature flag (Growth+ plans).
--
-- Design:
--   accounts        — chart of accounts, one row per account per agency.
--   journal_entries — the header of a double-entry journal entry.
--   journal_lines   — the debit/credit legs (must balance: SUM debit = SUM credit).
--
-- Enforcement of the balancing constraint is done in the application layer
-- (PostJournalEntryUseCase) so we can return a meaningful error, not a DB
-- exception.  The CHECK (debit_kes >= 0 AND credit_kes >= 0) guards against
-- negative amounts at the storage layer.

-- ── Chart of accounts ─────────────────────────────────────────────────────────

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
    created_at   TIMESTAMPTZ       NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ       NOT NULL DEFAULT now(),
    UNIQUE (agency_id, code)
);

CREATE INDEX idx_accounts_agency ON accounts (agency_id, account_type);

-- ── Journal entries ───────────────────────────────────────────────────────────

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

-- ── Journal lines ─────────────────────────────────────────────────────────────

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

-- ── updated_at trigger for accounts ──────────────────────────────────────────

CREATE TRIGGER trg_accounts_updated_at
    BEFORE UPDATE ON accounts
    FOR EACH ROW EXECUTE FUNCTION tenant_set_updated_at();

-- ── Seed system accounts (one set per agency on first use) ────────────────────
-- These rows are inserted by the application when provisioning an agency
-- (ProvisionAgencyUseCase), not here, because we don't know the agency_id
-- at migration time.