CREATE TABLE IF NOT EXISTS vendors (
    id           UUID        NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    agency_id    UUID        NOT NULL,
    name         TEXT        NOT NULL,
    contact_name TEXT,
    email        TEXT,
    phone        TEXT,
    speciality   TEXT,
    status       TEXT        NOT NULL DEFAULT 'active'
                             CHECK (status IN ('active','inactive','blacklisted')),
    notes        TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_vendors_agency ON vendors (agency_id, status);