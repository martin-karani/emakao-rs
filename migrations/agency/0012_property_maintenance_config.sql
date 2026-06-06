-- 0012_property_maintenance_config.sql
-- Moves work_order_prefix and work_order_seq to a separate table.

CREATE TABLE property_maintenance_configs (
    property_id       UUID         NOT NULL PRIMARY KEY REFERENCES properties (id) ON DELETE CASCADE,
    work_order_prefix TEXT         NOT NULL,
    work_order_seq    INT          NOT NULL DEFAULT 1,
    created_at        TIMESTAMPTZ  NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ  NOT NULL DEFAULT now()
);

-- Copy existing data
INSERT INTO property_maintenance_configs (property_id, work_order_prefix, work_order_seq)
SELECT id, work_order_prefix, work_order_seq FROM properties;

-- Remove columns from properties
ALTER TABLE properties DROP COLUMN work_order_prefix;
ALTER TABLE properties DROP COLUMN work_order_seq;
