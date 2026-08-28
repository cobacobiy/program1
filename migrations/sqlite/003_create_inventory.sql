-- Migration: 003_create_inventory.sql
CREATE TABLE IF NOT EXISTS inventory_stocks (
    product_id TEXT PRIMARY KEY,
    sku TEXT NOT NULL,
    product_name TEXT NOT NULL,
    image_url TEXT NOT NULL DEFAULT '',
    average_purchase_price REAL NOT NULL DEFAULT 0,
    warehouse_stock INTEGER NOT NULL DEFAULT 0,
    spare_stock INTEGER NOT NULL DEFAULT 0,
    locked_stock INTEGER NOT NULL DEFAULT 0,
    promotion_stock INTEGER NOT NULL DEFAULT 0,
    safety_stock INTEGER NOT NULL DEFAULT 0,
    available_stock INTEGER NOT NULL DEFAULT 0,
    last_updated TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS safety_stock_logs (
    id TEXT PRIMARY KEY,
    product_id TEXT NOT NULL,
    old_safety_stock INTEGER NOT NULL,
    new_safety_stock INTEGER NOT NULL,
    admin_note TEXT NOT NULL,
    updated_by TEXT NOT NULL DEFAULT 'System',
    timestamp TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
