-- Migration: 007_create_stock_adjustment_logs.sql
CREATE TABLE IF NOT EXISTS stock_adjustment_logs (
    id TEXT PRIMARY KEY,
    product_id TEXT NOT NULL,
    adjustment_type TEXT NOT NULL,
    old_value INTEGER NOT NULL,
    new_value INTEGER NOT NULL,
    admin_note TEXT NOT NULL,
    updated_by TEXT NOT NULL DEFAULT 'System',
    timestamp TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_stock_adj_product_id ON stock_adjustment_logs(product_id);
CREATE INDEX IF NOT EXISTS idx_stock_adj_type ON stock_adjustment_logs(adjustment_type);
CREATE INDEX IF NOT EXISTS idx_stock_adj_timestamp ON stock_adjustment_logs(timestamp);
