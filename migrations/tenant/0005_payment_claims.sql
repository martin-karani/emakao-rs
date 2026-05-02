CREATE TABLE IF NOT EXISTS payment_claims (
    id               UUID           NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    property_id      UUID           NOT NULL REFERENCES properties (id),
    agreement_id     UUID           REFERENCES agreements (id),
    resident_id      UUID           REFERENCES residents (id),
    method_type      TEXT           NOT NULL
                                    CHECK (method_type IN (
                                        'mpesa_paybill','mpesa_till',
                                        'bank_transfer','cash')),
    amount_kes       NUMERIC(14, 2) NOT NULL,
    reference_code   TEXT,
    proof_url        TEXT,
    notes            TEXT,
    status           TEXT           NOT NULL DEFAULT 'pending_review'
                                    CHECK (status IN ('pending_review','approved','rejected')),
    reviewed_by      UUID,
    reviewed_at      TIMESTAMPTZ,
    review_notes     TEXT,
    rejection_reason TEXT,
    ledger_entry_id  UUID,
    submitted_by     UUID           NOT NULL,
    created_at       TIMESTAMPTZ    NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ    NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_payment_claims_property
    ON payment_claims (property_id, status);
CREATE INDEX IF NOT EXISTS idx_payment_claims_agreement
    ON payment_claims (agreement_id);
CREATE INDEX IF NOT EXISTS idx_payment_claims_resident
    ON payment_claims (resident_id);