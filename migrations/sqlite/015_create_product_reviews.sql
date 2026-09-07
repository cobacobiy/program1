-- Migration: 015_create_product_reviews.sql
CREATE TABLE IF NOT EXISTS product_reviews (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES catalog_items(id) ON DELETE CASCADE,
    buyer_id TEXT NOT NULL REFERENCES buyer_accounts(id) ON DELETE CASCADE,
    order_id TEXT NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    rating INTEGER NOT NULL CHECK (rating >= 1 AND rating <= 5),
    review_text TEXT,
    is_visible INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(product_id, buyer_id, order_id)
);

CREATE INDEX IF NOT EXISTS idx_reviews_product ON product_reviews(product_id);
CREATE INDEX IF NOT EXISTS idx_reviews_buyer ON product_reviews(buyer_id);
CREATE INDEX IF NOT EXISTS idx_reviews_order ON product_reviews(order_id);
CREATE INDEX IF NOT EXISTS idx_reviews_product_visible ON product_reviews(product_id, is_visible);
CREATE INDEX IF NOT EXISTS idx_reviews_created_at ON product_reviews(created_at);
