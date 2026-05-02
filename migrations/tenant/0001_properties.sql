CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS properties (
    id            UUID        NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    agency_id     UUID        NOT NULL,
    name          TEXT        NOT NULL,
    address       TEXT        NOT NULL,
    city          TEXT        NOT NULL DEFAULT '',
    country_code  CHAR(2)     NOT NULL DEFAULT 'KE',
    property_type TEXT        NOT NULL,
    config        JSONB       NOT NULL DEFAULT '{}',
    created_by    UUID        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_properties_agency
    ON properties (agency_id);
CREATE INDEX IF NOT EXISTS idx_properties_type
    ON properties (agency_id, property_type);