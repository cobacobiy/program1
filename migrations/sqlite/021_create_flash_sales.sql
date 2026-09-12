-- Migration: 021_create_flash_sales.sql
CREATE TABLE IF NOT EXISTS flash_sale_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    start_time TEXT NOT NULL,
    end_time TEXT NOT NULL,
    banner_url TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS flash_sale_items (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL REFERENCES flash_sale_sessions(id) ON DELETE CASCADE,
    product_id TEXT NOT NULL REFERENCES catalog_items(id) ON DELETE CASCADE,
    flash_price_cents INTEGER NOT NULL,
    allocated_quantity INTEGER NOT NULL,
    sold_quantity INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_flash_sessions_active ON flash_sale_sessions(is_active, start_time, end_time);
CREATE UNIQUE INDEX IF NOT EXISTS idx_flash_item_session_product ON flash_sale_items(session_id, product_id);
