-- backend/migrations/agency/0004_property_billing_settings.sql

CREATE TABLE property_billing_settings (
    property_id         UUID           NOT NULL PRIMARY KEY REFERENCES properties (id) ON DELETE CASCADE,
    rent_due_day        INT            NOT NULL DEFAULT 5 CHECK (rent_due_day >= 1 AND rent_due_day <= 28),
    late_fee_type       TEXT           NOT NULL DEFAULT 'flat' CHECK (late_fee_type IN ('flat', 'percent')),
    late_fee_value      NUMERIC(14,2)  NOT NULL DEFAULT 0 CHECK (late_fee_value >= 0),
    late_fee_grace_days INT            NOT NULL DEFAULT 0 CHECK (late_fee_grace_days >= 0),
    
    -- Utility Constants
    water_rate_per_unit NUMERIC(14,2)  NOT NULL DEFAULT 0 CHECK (water_rate_per_unit >= 0),
    garbage_fee_kes     NUMERIC(14,2)  NOT NULL DEFAULT 0 CHECK (garbage_fee_kes >= 0),
    security_fee_kes    NUMERIC(14,2)  NOT NULL DEFAULT 0 CHECK (security_fee_kes >= 0),
    
    -- Flexible metadata for other fixed fees (e.g. {"parking": 1000})
    other_fixed_fees    JSONB          NOT NULL DEFAULT '{}',
    
    updated_at          TIMESTAMPTZ    NOT NULL DEFAULT now()
);

-- Initialise settings for existing properties by copying agency defaults or using defaults
INSERT INTO property_billing_settings (property_id)
SELECT id FROM properties
ON CONFLICT DO NOTHING;
