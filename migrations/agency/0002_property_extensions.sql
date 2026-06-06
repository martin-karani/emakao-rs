-- migrations/agency/0002_property_extensions.sql

-- 1. Add multifamily to property_type enum
ALTER TYPE property_type ADD VALUE IF NOT EXISTS 'multifamily';

-- 2. Add photos and documents to properties
ALTER TABLE properties ADD COLUMN IF NOT EXISTS photos JSONB NOT NULL DEFAULT '[]';
ALTER TABLE properties ADD COLUMN IF NOT EXISTS documents JSONB NOT NULL DEFAULT '[]';

-- 3. Create property_agents join table
CREATE TABLE IF NOT EXISTS property_agents (
    property_id UUID NOT NULL REFERENCES properties (id) ON DELETE CASCADE,
    user_id     UUID NOT NULL, -- References users(id) in platform DB
    agency_id   UUID NOT NULL, -- Sanity check
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (property_id, user_id)
);

CREATE INDEX idx_property_agents_user ON property_agents (user_id);
