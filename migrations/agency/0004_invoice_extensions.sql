-- migrations/agency/0004_invoice_extensions.sql

-- 1. invoices: add voided_at
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS voided_at TIMESTAMPTZ;

-- 2. work_orders: ensure code and work_order_number exist (safety check)
ALTER TABLE work_orders ADD COLUMN IF NOT EXISTS code TEXT UNIQUE;
ALTER TABLE work_orders ADD COLUMN IF NOT EXISTS work_order_number INTEGER;
