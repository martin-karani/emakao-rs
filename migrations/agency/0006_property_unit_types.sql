-- backend/migrations/agency/0006_property_unit_types.sql

ALTER TABLE properties ADD COLUMN unit_types JSONB NOT NULL DEFAULT '[]';
