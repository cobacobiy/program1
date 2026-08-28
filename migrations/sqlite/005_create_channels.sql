-- Migration: 005_create_channels.sql
CREATE TABLE IF NOT EXISTS channel_statuses (
    channel TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    synced_products INTEGER NOT NULL DEFAULT 0,
    total_sales REAL NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    last_synced_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
