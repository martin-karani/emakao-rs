-- 20. custom_roles
CREATE TABLE custom_roles (
    id UUID PRIMARY KEY,
    agency_id UUID NOT NULL,
    name TEXT NOT NULL,
    permissions JSONB NOT NULL,
    is_system BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agency_id, name)
);
CREATE INDEX idx_custom_roles_agency ON custom_roles (agency_id);
