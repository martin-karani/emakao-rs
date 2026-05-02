CREATE TABLE IF NOT EXISTS agreements (
    id                UUID           NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    property_id       UUID           NOT NULL REFERENCES properties (id),
    unit_id           UUID           NOT NULL REFERENCES units (id),
    resident_id       UUID           NOT NULL REFERENCES residents (id),
    start_date        DATE           NOT NULL,
    end_date          DATE,
    rent_amount_kes   NUMERIC(14, 2) NOT NULL,
    deposit_kes       NUMERIC(14, 2) NOT NULL DEFAULT 0,
    billing_frequency TEXT           NOT NULL DEFAULT 'monthly'
                                     CHECK (billing_frequency IN (
                                         'monthly','quarterly','semi_annual','annual')),
    status            TEXT           NOT NULL DEFAULT 'active'
                                     CHECK (status IN (
                                         'active','expired','terminated','pending_renewal')),
    created_at        TIMESTAMPTZ    NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ    NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_agreements_unit      ON agreements (unit_id, status);
CREATE INDEX IF NOT EXISTS idx_agreements_resident  ON agreements (resident_id);
CREATE INDEX IF NOT EXISTS idx_agreements_property  ON agreements (property_id);

-- Prevent two active agreements on the same unit
CREATE UNIQUE INDEX IF NOT EXISTS idx_agreements_active_unit
    ON agreements (unit_id)
    WHERE status = 'active';

-- Auto-update unit status when agreement status changes
CREATE OR REPLACE FUNCTION sync_unit_status()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.status = 'active' THEN
        UPDATE units SET status = 'occupied', updated_at = now()
        WHERE id = NEW.unit_id;
    ELSIF NEW.status IN ('terminated', 'expired') THEN
        UPDATE units SET status = 'vacant', updated_at = now()
        WHERE id = NEW.unit_id;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_agreement_unit_status
    AFTER INSERT OR UPDATE OF status ON agreements
    FOR EACH ROW EXECUTE FUNCTION sync_unit_status();