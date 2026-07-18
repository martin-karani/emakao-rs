-- Property operating expenses captured by staff for a specific property.

CREATE TABLE property_expenses (
    id            UUID          NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id     UUID          NOT NULL,
    property_id   UUID          NOT NULL REFERENCES properties (id) ON DELETE CASCADE,
    expense_date  DATE          NOT NULL,
    category      TEXT          NOT NULL CHECK (
        category IN (
            'maintenance',
            'security',
            'utilities',
            'payroll',
            'management',
            'taxes',
            'insurance',
            'supplies',
            'bank_charges',
            'other'
        )
    ),
    description   TEXT          NOT NULL CHECK (length(trim(description)) > 0),
    vendor_name   TEXT,
    amount_kes    NUMERIC(14,2) NOT NULL CHECK (amount_kes > 0),
    notes         TEXT,
    created_by    UUID          NOT NULL,
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ   NOT NULL DEFAULT now()
);

CREATE INDEX idx_property_expenses_property_date
    ON property_expenses (property_id, expense_date DESC, created_at DESC);

CREATE INDEX idx_property_expenses_agency_date
    ON property_expenses (agency_id, expense_date DESC, created_at DESC);
