CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS agencies (
    id           UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    name         TEXT        NOT NULL,
    slug         TEXT        NOT NULL UNIQUE,
    schema_name  TEXT        NOT NULL UNIQUE,
    country_code CHAR(2)     NOT NULL DEFAULT 'KE',
    currency_code CHAR(3)    NOT NULL DEFAULT 'KES',
    status        TEXT       NOT NULL DEFAULT 'active',
    fga_store_id TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_agencies_slug ON agencies (slug);