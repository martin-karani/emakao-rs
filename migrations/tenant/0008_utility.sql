CREATE TABLE IF NOT EXISTS utility_meters (
    id           UUID           NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    unit_id      UUID           NOT NULL REFERENCES units (id) ON DELETE CASCADE,
    meter_type   TEXT           NOT NULL CHECK (meter_type IN ('electricity','water','gas')),
    billing_mode TEXT           NOT NULL CHECK (billing_mode IN ('prepaid','postpaid')),
    meter_number TEXT           NOT NULL,
    rate_per_unit NUMERIC(10,4) NOT NULL,
    created_at   TIMESTAMPTZ    NOT NULL DEFAULT now(),
    UNIQUE (unit_id, meter_type)
);

CREATE TABLE IF NOT EXISTS meter_readings (
    id            UUID           NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    meter_id      UUID           NOT NULL REFERENCES utility_meters (id) ON DELETE CASCADE,
    reading_value NUMERIC(14, 4) NOT NULL,
    read_at       TIMESTAMPTZ    NOT NULL DEFAULT now(),
    recorded_by   UUID           NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_meter_readings_meter
    ON meter_readings (meter_id, read_at DESC);

CREATE TABLE IF NOT EXISTS utility_bills (
    id             UUID           NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    meter_id       UUID           NOT NULL REFERENCES utility_meters (id),
    unit_id        UUID           NOT NULL REFERENCES units (id),
    units_consumed NUMERIC(14, 4) NOT NULL,
    amount_kes     NUMERIC(14, 2) NOT NULL,
    status         TEXT           NOT NULL DEFAULT 'draft'
                                  CHECK (status IN ('draft','issued','paid','overdue')),
    created_at     TIMESTAMPTZ    NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_utility_bills_unit
    ON utility_bills (unit_id, status);