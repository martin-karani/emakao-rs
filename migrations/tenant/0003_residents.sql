CREATE TABLE IF NOT EXISTS residents (
    id            UUID        NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id       UUID        NOT NULL,
    first_name    TEXT        NOT NULL,
    last_name     TEXT        NOT NULL,
    email         TEXT        NOT NULL,
    phone         TEXT,
    national_id   TEXT,
    portal_status TEXT        NOT NULL DEFAULT 'invited'
                              CHECK (portal_status IN ('invited','active','suspended')),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_residents_email    ON residents (email);
CREATE INDEX IF NOT EXISTS idx_residents_user_id  ON residents (user_id);