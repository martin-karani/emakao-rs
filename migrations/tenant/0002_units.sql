CREATE TABLE IF NOT EXISTS units (
    id              UUID           NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    property_id     UUID           NOT NULL REFERENCES properties (id) ON DELETE CASCADE,
    unit_number     TEXT           NOT NULL,
    floor           INTEGER,
    size_sqm        DOUBLE PRECISION,
    bedrooms        SMALLINT,
    bathrooms       SMALLINT,
    rent_amount_kes NUMERIC(14, 2) NOT NULL DEFAULT 0,
    deposit_kes     NUMERIC(14, 2) NOT NULL DEFAULT 0,
    status          TEXT           NOT NULL DEFAULT 'vacant'
                                   CHECK (status IN ('vacant','occupied','maintenance','reserved')),
    description     TEXT,
    photos          JSONB          NOT NULL DEFAULT '[]',
    created_at      TIMESTAMPTZ    NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ    NOT NULL DEFAULT now(),

    UNIQUE (property_id, unit_number)
);

CREATE INDEX IF NOT EXISTS idx_units_property  ON units (property_id);
CREATE INDEX IF NOT EXISTS idx_units_status    ON units (property_id, status);