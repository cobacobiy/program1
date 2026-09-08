-- Migration 020: Create Return Requests Table
CREATE TABLE IF NOT EXISTS return_requests (
    id TEXT PRIMARY KEY NOT NULL,
    order_id TEXT NOT NULL REFERENCES orders(id),
    buyer_id TEXT NOT NULL REFERENCES buyer_accounts(id),
    reason TEXT NOT NULL,              -- 'defective', 'wrong_item', 'not_as_described', 'other'
    description TEXT,
    evidence_urls TEXT,                -- JSON array of image URLs
    status TEXT NOT NULL DEFAULT 'pending', -- pending, approved, rejected, return_shipped, received, refunded
    refund_amount_cents INTEGER NOT NULL,
    admin_notes TEXT,
    processed_by TEXT,                 -- admin username/user_id
    processed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_returns_order ON return_requests(order_id);
CREATE INDEX IF NOT EXISTS idx_returns_buyer ON return_requests(buyer_id);
CREATE INDEX IF NOT EXISTS idx_returns_status ON return_requests(status);
