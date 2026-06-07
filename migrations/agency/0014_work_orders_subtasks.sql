-- Add subtasks JSONB column to work_orders
ALTER TABLE work_orders 
ADD COLUMN subtasks JSONB NOT NULL DEFAULT '[]'::jsonb;
