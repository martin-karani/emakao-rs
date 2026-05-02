CREATE TABLE IF NOT EXISTS ledger_entries (
    id            UUID           NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    agreement_id  UUID           REFERENCES agreements (id),
    unit_id       UUID           REFERENCES units (id),
    resident_id   UUID           REFERENCES residents (id),
    owner_id      UUID,
    entry_type    TEXT           NOT NULL,
    amount_kes    NUMERIC(14, 2) NOT NULL,
    description   TEXT           NOT NULL,
    external_ref  TEXT,
    mpesa_receipt TEXT,
    period_start  DATE,
    period_end    DATE,
    posted_by     UUID           NOT NULL,
    posted_at     TIMESTAMPTZ    NOT NULL DEFAULT now(),
    is_reconciled BOOLEAN        NOT NULL DEFAULT false,
    metadata      JSONB          NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_ledger_agreement
    ON ledger_entries (agreement_id, posted_at DESC);
CREATE INDEX IF NOT EXISTS idx_ledger_unit
    ON ledger_entries (unit_id);
CREATE INDEX IF NOT EXISTS idx_ledger_resident
    ON ledger_entries (resident_id);
CREATE INDEX IF NOT EXISTS idx_ledger_type
    ON ledger_entries (entry_type, posted_at DESC);
CREATE INDEX IF NOT EXISTS idx_ledger_mpesa
    ON ledger_entries (mpesa_receipt)
    WHERE mpesa_receipt IS NOT NULL;