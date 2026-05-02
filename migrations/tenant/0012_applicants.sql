CREATE TABLE IF NOT EXISTS applicants (
    id                  UUID           NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    agency_id           UUID           NOT NULL,
    property_id         UUID           NOT NULL REFERENCES properties (id),
    unit_id             UUID           REFERENCES units (id),
    first_name          TEXT           NOT NULL,
    last_name           TEXT           NOT NULL,
    email               TEXT           NOT NULL,
    phone               TEXT,
    national_id         TEXT,
    monthly_income_kes  NUMERIC(14, 2),
    employer            TEXT,
    status              TEXT           NOT NULL DEFAULT 'submitted'
                                       CHECK (status IN (
                                           'submitted','under_review',
                                           'approved','rejected','withdrawn')),
    notes               TEXT,
    created_at          TIMESTAMPTZ    NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ    NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_applicants_property
    ON applicants (property_id, status);