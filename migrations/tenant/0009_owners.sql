CREATE TABLE IF NOT EXISTS owners (
    id           UUID        NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id      UUID,
    first_name   TEXT        NOT NULL,
    last_name    TEXT        NOT NULL,
    email        TEXT        NOT NULL,
    phone        TEXT,
    company_name TEXT,
    kra_pin      TEXT,
    bank_name    TEXT,
    bank_account TEXT,
    mpesa_number TEXT,
    portal_status TEXT       NOT NULL DEFAULT 'invited'
                             CHECK (portal_status IN ('invited','active','suspended')),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS property_owners (
    property_id       UUID        NOT NULL REFERENCES properties (id) ON DELETE CASCADE,
    owner_id          UUID        NOT NULL REFERENCES owners (id) ON DELETE CASCADE,
    ownership_percent NUMERIC(5,2) NOT NULL DEFAULT 100,
    PRIMARY KEY (property_id, owner_id)
);

CREATE INDEX IF NOT EXISTS idx_owners_email ON owners (email);