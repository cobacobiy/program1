-- Migration: 016_create_wishlists.sql
CREATE TABLE IF NOT EXISTS buyer_wishlists (
    id TEXT PRIMARY KEY NOT NULL,
    buyer_id TEXT NOT NULL REFERENCES buyer_accounts(id) ON DELETE CASCADE,
    product_id TEXT NOT NULL REFERENCES catalog_items(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(buyer_id, product_id)
);

CREATE INDEX IF NOT EXISTS idx_wishlist_buyer ON buyer_wishlists(buyer_id);
CREATE INDEX IF NOT EXISTS idx_wishlist_product ON buyer_wishlists(product_id);
