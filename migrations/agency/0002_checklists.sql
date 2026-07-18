
-- =============================================================================
-- Checklists — adds tables for move-in/move-out checklists
-- =============================================================================

CREATE TYPE checklist_type AS ENUM (
    'move-in',
    'move-out',
    'both'
);

CREATE TABLE checklists (
    id UUID NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    name TEXT NOT NULL CHECK (char_length(trim(name)) > 0),
    description TEXT,
    is_default BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_checklists_is_default ON checklists(is_default);

CREATE TABLE checklist_sections (
    id UUID NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    checklist_id UUID NOT NULL REFERENCES checklists(id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (char_length(trim(name)) > 0),
    description TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_checklist_sections_checklist_id ON checklist_sections(checklist_id);
CREATE INDEX idx_checklist_sections_sort_order ON checklist_sections(sort_order);

CREATE TABLE checklist_items (
    id UUID NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    section_id UUID NOT NULL REFERENCES checklist_sections(id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (char_length(trim(name)) > 0),
    description TEXT,
    checklist_type checklist_type NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_checklist_items_section_id ON checklist_items(section_id);
CREATE INDEX idx_checklist_items_sort_order ON checklist_items(sort_order);

CREATE TABLE property_checklists (
    id UUID NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    property_id UUID NOT NULL REFERENCES properties(id) ON DELETE CASCADE,
    checklist_id UUID NOT NULL REFERENCES checklists(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(property_id, checklist_id)
);

CREATE INDEX idx_property_checklists_property_id ON property_checklists(property_id);
CREATE INDEX idx_property_checklists_checklist_id ON property_checklists(checklist_id);

-- Add updated_at triggers for all checklist tables
DO $$
DECLARE
    tbl TEXT;
BEGIN
    FOREACH tbl IN ARRAY ARRAY['checklists', 'checklist_sections', 'checklist_items', 'property_checklists'] LOOP
        EXECUTE format(
            'CREATE TRIGGER trg_%s_updated_at
             BEFORE UPDATE ON %s
             FOR EACH ROW EXECUTE FUNCTION tenant_set_updated_at();',
            tbl, tbl
        );
    END LOOP;
END;
$$;
