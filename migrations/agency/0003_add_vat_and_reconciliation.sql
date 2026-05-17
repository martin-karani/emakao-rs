-- migrations/agency/0003_add_vat_and_reconciliation.sql
--
-- Extends the accounting schema with:
--   1.  `vat_applicable` + `vat_rate` columns on `accounts` — marks which
--       accounts generate VAT entries and at what rate (default 16 %).
--   2.  `bank_statements` + `bank_statement_lines` tables for the bank
--       reconciliation feature.

-- ── VAT columns on accounts 

ALTER TABLE accounts
    ADD COLUMN IF NOT EXISTS vat_applicable BOOLEAN       NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS vat_rate       NUMERIC(5, 4)          DEFAULT 0.1600;
-- vat_rate is NULL when vat_applicable = false.
-- CHECK enforced in the application layer (not DB) to keep error messages clear.

COMMENT ON COLUMN accounts.vat_applicable IS
    'True for accounts that generate output VAT (revenue) or input VAT (expense) entries.';
COMMENT ON COLUMN accounts.vat_rate IS
    'Fractional VAT rate, e.g. 0.1600 = 16 %. NULL when vat_applicable = false.';

-- ── Bank reconciliation 

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