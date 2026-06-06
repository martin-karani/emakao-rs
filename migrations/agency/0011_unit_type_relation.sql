-- backend/migrations/agency/0011_unit_type_relation.sql

ALTER TABLE units ADD COLUMN unit_type_id UUID;

CREATE INDEX idx_units_unit_type ON units (unit_type_id) WHERE unit_type_id IS NOT NULL;

-- Note: We don't add a hard FK constraint to the property JSONB, 
-- but application logic should ensure unit_type_id matches an ID in property.unit_types
