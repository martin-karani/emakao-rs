-- Migration: Add soft delete support to properties and allow duplicate names
-- Created: 2025-01-20

-- 1. Add deleted_at column for soft delete
ALTER TABLE properties ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ;

-- 2. Add index for filtering active properties
CREATE INDEX IF NOT EXISTS idx_properties_not_deleted 
ON properties (agency_id, deleted_at) 
WHERE deleted_at IS NULL;

-- 3. Create partial unique index for active property names
-- This allows duplicate names only for deleted properties
DROP INDEX IF EXISTS uq_properties_agency_name;
CREATE UNIQUE INDEX uq_properties_agency_active_name 
ON properties (agency_id, name) 
WHERE deleted_at IS NULL;

-- 4. Create function to soft delete property
CREATE OR REPLACE FUNCTION soft_delete_property(property_id UUID)
RETURNS VOID AS $$
BEGIN
    -- Mark property as deleted
    UPDATE properties 
    SET deleted_at = NOW(), 
        updated_at = NOW()
    WHERE id = property_id 
    AND deleted_at IS NULL;
    
    -- Note: Related records (units, leases, etc.) remain intact
    -- for audit and reporting purposes. They can be cleaned up 
    -- later via a background job if needed.
END;
$$ LANGUAGE plpgsql;

-- 5. Create view for active properties only
CREATE OR REPLACE VIEW active_properties AS
SELECT * FROM properties WHERE deleted_at IS NULL;

-- Comment explaining the soft delete behavior
COMMENT ON COLUMN properties.deleted_at IS 
'Timestamp when property was soft deleted. NULL = active property. Soft deleted properties are hidden from normal API responses but preserved for audit.';
