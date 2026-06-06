-- backend/migrations/agency/0003_property_level_notifications.sql

ALTER TABLE notification_templates ADD COLUMN property_id UUID REFERENCES properties (id) ON DELETE CASCADE;

-- Update unique constraint to include property_id
-- We drop the old one and create a new one. 
-- Note: In Postgres, multiple NULLs are allowed in a UNIQUE constraint unless specified otherwise.
-- However, for templates, we want at most one agency-wide template (property_id IS NULL)
-- and at most one per-property template.
ALTER TABLE notification_templates DROP CONSTRAINT notification_templates_agency_id_channel_event_key_locale_key;

-- Use a single unique index that treats NULL property_id as a specific UUID for uniqueness
CREATE UNIQUE INDEX idx_notif_templates_upsert_unique 
ON notification_templates (agency_id, COALESCE(property_id, '00000000-0000-0000-0000-000000000000'), channel, event_key, locale);
