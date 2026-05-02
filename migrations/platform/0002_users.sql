CREATE TABLE IF NOT EXISTS users (
    id            UUID        NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    agency_id     UUID        NOT NULL REFERENCES agencies (id) ON DELETE CASCADE,
    email         TEXT        NOT NULL,
    password_hash TEXT        NOT NULL DEFAULT '',
    role          TEXT        NOT NULL DEFAULT 'agent'
                              CHECK (role IN ('admin', 'manager', 'agent', 'resident', 'owner')),
    is_active     BOOLEAN     NOT NULL DEFAULT true,
    last_login_at TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (agency_id, email)
);

CREATE INDEX IF NOT EXISTS idx_users_agency_email ON users (agency_id, email);