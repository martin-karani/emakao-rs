CREATE TABLE IF NOT EXISTS inspections (
    id              UUID        NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    property_id     UUID        NOT NULL REFERENCES properties (id),
    unit_id         UUID        NOT NULL REFERENCES units (id),
    agreement_id    UUID        REFERENCES agreements (id),
    inspection_type TEXT        NOT NULL
                                CHECK (inspection_type IN
                                    ('move_in','move_out','routine','emergency')),
    status          TEXT        NOT NULL DEFAULT 'scheduled'
                                CHECK (status IN
                                    ('scheduled','in_progress','completed','cancelled')),
    scheduled_at    TIMESTAMPTZ NOT NULL,
    completed_at    TIMESTAMPTZ,
    conducted_by    UUID,
    items           JSONB       NOT NULL DEFAULT '[]',
    summary_notes   TEXT,
    created_by      UUID        NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_inspections_unit
    ON inspections (unit_id, status);
CREATE INDEX IF NOT EXISTS idx_inspections_scheduled
    ON inspections (scheduled_at)
    WHERE status = 'scheduled';