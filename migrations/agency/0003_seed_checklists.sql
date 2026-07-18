-- =============================================================================
-- Seed default checklist templates
-- These are global templates (not linked to any property) that admins can use
-- as a starting point when creating property checklists.
-- =============================================================================

DO $$
DECLARE
    move_in_id  UUID := uuidv7();
    move_out_id UUID := uuidv7();

    -- move-in sections
    s_keys_id         UUID := uuidv7();
    s_exterior_id     UUID := uuidv7();
    s_living_id       UUID := uuidv7();
    s_kitchen_id      UUID := uuidv7();
    s_bathroom_id     UUID := uuidv7();
    s_bedroom_id      UUID := uuidv7();
    s_utilities_id    UUID := uuidv7();

    -- move-out sections (reuse names but different UUIDs)
    so_keys_id        UUID := uuidv7();
    so_exterior_id    UUID := uuidv7();
    so_living_id      UUID := uuidv7();
    so_kitchen_id     UUID := uuidv7();
    so_bathroom_id    UUID := uuidv7();
    so_bedroom_id     UUID := uuidv7();
    so_utilities_id   UUID := uuidv7();
BEGIN

-- ─── Move-In Checklist ────────────────────────────────────────────────────────
INSERT INTO checklists (id, name, description, is_default)
VALUES (
    move_in_id,
    'Standard Move-In',
    'Default move-in inspection checklist covering all areas of a residential unit.',
    true
);

-- Sections
INSERT INTO checklist_sections (id, checklist_id, name, sort_order)
VALUES
    (s_keys_id,      move_in_id, 'Keys & Access',         0),
    (s_exterior_id,  move_in_id, 'Exterior & Entry',      1),
    (s_living_id,    move_in_id, 'Living & Dining Areas', 2),
    (s_kitchen_id,   move_in_id, 'Kitchen',               3),
    (s_bathroom_id,  move_in_id, 'Bathrooms',             4),
    (s_bedroom_id,   move_in_id, 'Bedrooms',              5),
    (s_utilities_id, move_in_id, 'Utilities & Safety',    6);

-- Items
INSERT INTO checklist_items (id, section_id, name, checklist_type, sort_order)
VALUES
    -- Keys & Access
    (uuidv7(), s_keys_id, 'All keys provided to tenant', 'move-in', 0),
    (uuidv7(), s_keys_id, 'Gate remote / access card working', 'move-in', 1),
    (uuidv7(), s_keys_id, 'Mailbox key provided', 'move-in', 2),

    -- Exterior & Entry
    (uuidv7(), s_exterior_id, 'Front door locks and handles functional', 'move-in', 0),
    (uuidv7(), s_exterior_id, 'Exterior walls free of major damage', 'move-in', 1),
    (uuidv7(), s_exterior_id, 'Windows in good condition (no broken panes)', 'move-in', 2),
    (uuidv7(), s_exterior_id, 'Parking area noted and in good condition', 'move-in', 3),

    -- Living & Dining Areas
    (uuidv7(), s_living_id, 'Walls and ceiling free of cracks or stains', 'move-in', 0),
    (uuidv7(), s_living_id, 'Floors clean and undamaged', 'move-in', 1),
    (uuidv7(), s_living_id, 'Electrical outlets working', 'move-in', 2),
    (uuidv7(), s_living_id, 'Light fixtures working', 'move-in', 3),
    (uuidv7(), s_living_id, 'Windows open/close properly', 'move-in', 4),
    (uuidv7(), s_living_id, 'Window locks functional', 'move-in', 5),

    -- Kitchen
    (uuidv7(), s_kitchen_id, 'Sink drains freely, no leaks', 'move-in', 0),
    (uuidv7(), s_kitchen_id, 'Tap water flows hot and cold', 'move-in', 1),
    (uuidv7(), s_kitchen_id, 'Cabinets and drawers functional', 'move-in', 2),
    (uuidv7(), s_kitchen_id, 'Countertops clean and undamaged', 'move-in', 3),
    (uuidv7(), s_kitchen_id, 'Cooking appliances working (if provided)', 'move-in', 4),
    (uuidv7(), s_kitchen_id, 'Exhaust fan functional', 'move-in', 5),

    -- Bathrooms
    (uuidv7(), s_bathroom_id, 'Toilet flushes properly, no leaks', 'move-in', 0),
    (uuidv7(), s_bathroom_id, 'Shower / bath drains freely', 'move-in', 1),
    (uuidv7(), s_bathroom_id, 'Taps functional, hot and cold water', 'move-in', 2),
    (uuidv7(), s_bathroom_id, 'Tiles and grouting in good condition', 'move-in', 3),
    (uuidv7(), s_bathroom_id, 'Exhaust fan or window ventilation present', 'move-in', 4),
    (uuidv7(), s_bathroom_id, 'Mirror(s) intact', 'move-in', 5),

    -- Bedrooms
    (uuidv7(), s_bedroom_id, 'Walls and ceiling clean and undamaged', 'move-in', 0),
    (uuidv7(), s_bedroom_id, 'Floors clean and undamaged', 'move-in', 1),
    (uuidv7(), s_bedroom_id, 'Built-in wardrobes / closets functional', 'move-in', 2),
    (uuidv7(), s_bedroom_id, 'Electrical outlets working', 'move-in', 3),
    (uuidv7(), s_bedroom_id, 'Light fixtures working', 'move-in', 4),
    (uuidv7(), s_bedroom_id, 'Windows open/close and lock properly', 'move-in', 5),

    -- Utilities & Safety
    (uuidv7(), s_utilities_id, 'Electricity meter reading noted', 'move-in', 0),
    (uuidv7(), s_utilities_id, 'Water meter reading noted', 'move-in', 1),
    (uuidv7(), s_utilities_id, 'Smoke detector present and tested', 'move-in', 2),
    (uuidv7(), s_utilities_id, 'Fire extinguisher present (if applicable)', 'move-in', 3),
    (uuidv7(), s_utilities_id, 'Main electricity breaker accessible', 'move-in', 4),
    (uuidv7(), s_utilities_id, 'Main water shut-off accessible', 'move-in', 5);


-- ─── Move-Out Checklist ───────────────────────────────────────────────────────
INSERT INTO checklists (id, name, description, is_default)
VALUES (
    move_out_id,
    'Standard Move-Out',
    'Default move-out inspection checklist to verify the unit condition at the end of a tenancy.',
    true
);

-- Sections
INSERT INTO checklist_sections (id, checklist_id, name, sort_order)
VALUES
    (so_keys_id,      move_out_id, 'Keys & Access',         0),
    (so_exterior_id,  move_out_id, 'Exterior & Entry',      1),
    (so_living_id,    move_out_id, 'Living & Dining Areas', 2),
    (so_kitchen_id,   move_out_id, 'Kitchen',               3),
    (so_bathroom_id,  move_out_id, 'Bathrooms',             4),
    (so_bedroom_id,   move_out_id, 'Bedrooms',              5),
    (so_utilities_id, move_out_id, 'Utilities & Safety',    6);

-- Items
INSERT INTO checklist_items (id, section_id, name, checklist_type, sort_order)
VALUES
    -- Keys & Access
    (uuidv7(), so_keys_id, 'All keys returned by tenant', 'move-out', 0),
    (uuidv7(), so_keys_id, 'Gate remote / access card returned', 'move-out', 1),
    (uuidv7(), so_keys_id, 'Mailbox key returned', 'move-out', 2),

    -- Exterior & Entry
    (uuidv7(), so_exterior_id, 'Front door locks and handles functional', 'move-out', 0),
    (uuidv7(), so_exterior_id, 'Exterior walls checked for new damage', 'move-out', 1),
    (uuidv7(), so_exterior_id, 'Windows intact (no new broken panes)', 'move-out', 2),

    -- Living & Dining Areas
    (uuidv7(), so_living_id, 'Walls clean — no unrepaired holes or marks', 'move-out', 0),
    (uuidv7(), so_living_id, 'Floors clean and undamaged', 'move-out', 1),
    (uuidv7(), so_living_id, 'Electrical outlets working', 'move-out', 2),
    (uuidv7(), so_living_id, 'Light fixtures working', 'move-out', 3),
    (uuidv7(), so_living_id, 'Windows and locks functional', 'move-out', 4),

    -- Kitchen
    (uuidv7(), so_kitchen_id, 'Sink clean, drains freely, no leaks', 'move-out', 0),
    (uuidv7(), so_kitchen_id, 'Cabinets and drawers clean and functional', 'move-out', 1),
    (uuidv7(), so_kitchen_id, 'Countertops clean and undamaged', 'move-out', 2),
    (uuidv7(), so_kitchen_id, 'Appliances clean (if provided)', 'move-out', 3),

    -- Bathrooms
    (uuidv7(), so_bathroom_id, 'Toilet clean and functional', 'move-out', 0),
    (uuidv7(), so_bathroom_id, 'Shower / bath clean, drains freely', 'move-out', 1),
    (uuidv7(), so_bathroom_id, 'Tiles and grouting intact', 'move-out', 2),
    (uuidv7(), so_bathroom_id, 'No mould or damage', 'move-out', 3),

    -- Bedrooms
    (uuidv7(), so_bedroom_id, 'Walls clean — no unrepaired damage', 'move-out', 0),
    (uuidv7(), so_bedroom_id, 'Floors clean and undamaged', 'move-out', 1),
    (uuidv7(), so_bedroom_id, 'Built-in wardrobes / closets functional and clean', 'move-out', 2),
    (uuidv7(), so_bedroom_id, 'Electrical outlets working', 'move-out', 3),
    (uuidv7(), so_bedroom_id, 'Light fixtures working', 'move-out', 4),

    -- Utilities & Safety
    (uuidv7(), so_utilities_id, 'Electricity meter reading noted', 'move-out', 0),
    (uuidv7(), so_utilities_id, 'Water meter reading noted', 'move-out', 1),
    (uuidv7(), so_utilities_id, 'Smoke detector present and operational', 'move-out', 2),
    (uuidv7(), so_utilities_id, 'Unit fully vacated and clean', 'move-out', 3);

END $$;
