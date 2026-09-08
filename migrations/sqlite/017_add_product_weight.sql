-- Migration: 017_add_product_weight.sql
-- Add weight_grams to catalog_items and courier/shipping_cost to orders

ALTER TABLE catalog_items ADD COLUMN weight_grams INTEGER NOT NULL DEFAULT 500;
ALTER TABLE orders ADD COLUMN courier TEXT;
ALTER TABLE orders ADD COLUMN shipping_cost_cents INTEGER NOT NULL DEFAULT 0;
