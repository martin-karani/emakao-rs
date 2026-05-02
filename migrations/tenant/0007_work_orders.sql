CREATE TABLE IF NOT EXISTS work_orders (
    id          UUID        NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    property_id UUID        NOT NULL REFERENCES properties (id),
    unit_id     UUID        REFERENCES units (id),
    title       TEXT        NOT NULL,
    description TEXT,
    status      TEXT        NOT NULL DEFAULT 'open'
                            CHECK (status IN ('open','in_progress','completed','cancelled')),
    priority    TEXT        NOT NULL DEFAULT 'medium'
                            CHECK (priority IN ('low','medium','high','emergency')),
    vendor_id   UUID,
    reported_by UUID        NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_work_orders_property
    ON work_orders (property_id, status);
CREATE INDEX IF NOT EXISTS idx_work_orders_unit
    ON work_orders (unit_id)
    WHERE unit_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_work_orders_priority
    ON work_orders (priority, created_at DESC)
    WHERE status NOT IN ('completed','cancelled');