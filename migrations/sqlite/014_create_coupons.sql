-- Migration: 014_create_coupons.sql
CREATE TABLE IF NOT EXISTS coupons (
    id TEXT PRIMARY KEY NOT NULL,
    code TEXT NOT NULL UNIQUE,                -- Always UPPERCASE
    description TEXT,
    discount_type TEXT NOT NULL DEFAULT 'percentage', -- 'percentage' or 'fixed'
    discount_value REAL NOT NULL,             -- percentage (e.g. 20.0 for 20%) or fixed discount amount (e.g. 50000.0)
    min_order_amount REAL NOT NULL DEFAULT 0.0,
    max_discount_amount REAL,                 -- Cap on percentage discount
    usage_limit INTEGER,                      -- NULL = unlimited
    usage_count INTEGER NOT NULL DEFAULT 0,
    valid_from TEXT NOT NULL,
    valid_until TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS coupon_usages (
    id TEXT PRIMARY KEY NOT NULL,
    coupon_id TEXT NOT NULL REFERENCES coupons(id) ON DELETE CASCADE,
    buyer_id TEXT,
    order_id TEXT,
    discount_amount REAL NOT NULL,
    used_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_coupons_code ON coupons(code);
CREATE INDEX IF NOT EXISTS idx_coupon_usages_coupon ON coupon_usages(coupon_id);
