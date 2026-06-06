-- backend/migrations/agency/0005_property_billing_currency.sql

ALTER TABLE property_billing_settings ADD COLUMN currency_code CHAR(3) NOT NULL DEFAULT 'KES';
