-- Migration: 0013_property_policies.sql
-- Adds a JSONB `policies` column to store per-property payment methods,
-- agent commission, late-fee overrides, deposit rules, and service charges.
-- The billing_settings table continues to drive the actual charge calculations;
-- `policies` is the UI-editable configuration surface.

ALTER TABLE properties
    ADD COLUMN IF NOT EXISTS policies JSONB;

COMMENT ON COLUMN properties.policies IS
    'Per-property configuration blob: payment_methods, agent_commission_percent, '
    'deposit rules, and service_charges for multi-unit properties.';
