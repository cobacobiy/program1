-- Migration: 022_create_loyalty_points.sql
ALTER TABLE buyer_accounts ADD COLUMN points_balance INTEGER NOT NULL DEFAULT 0;
ALTER TABLE buyer_accounts ADD COLUMN membership_tier TEXT NOT NULL DEFAULT 'Classic';

CREATE TABLE IF NOT EXISTS loyalty_point_ledgers (
    id TEXT PRIMARY KEY NOT NULL,
    buyer_id TEXT NOT NULL REFERENCES buyer_accounts(id) ON DELETE CASCADE,
    order_id TEXT REFERENCES orders(id) ON DELETE SET NULL,
    points_delta INTEGER NOT NULL,
    balance_after INTEGER NOT NULL,
    description TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_loyalty_buyer ON loyalty_point_ledgers(buyer_id, created_at DESC);

ALTER TABLE orders ADD COLUMN points_redeemed INTEGER NOT NULL DEFAULT 0;
