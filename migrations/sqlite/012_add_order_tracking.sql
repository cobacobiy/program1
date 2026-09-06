-- Migration: 012_add_order_tracking.sql
-- Add order tracking, completion, and cancellation fields to orders table

ALTER TABLE orders ADD COLUMN tracking_number TEXT;
ALTER TABLE orders ADD COLUMN shipped_at TEXT;
ALTER TABLE orders ADD COLUMN delivered_at TEXT;
ALTER TABLE orders ADD COLUMN cancelled_at TEXT;
ALTER TABLE orders ADD COLUMN cancelled_by TEXT;
ALTER TABLE orders ADD COLUMN cancel_reason TEXT;

CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status);
