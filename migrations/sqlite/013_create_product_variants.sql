-- Migration: 013_create_product_variants.sql
CREATE TABLE IF NOT EXISTS product_variants (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES catalog_items(id) ON DELETE CASCADE,
    variant_name TEXT NOT NULL,          -- e.g. "Ukuran", "Warna"
    variant_value TEXT NOT NULL,         -- e.g. "XL", "Navy Blue"
    sku TEXT,                            -- optional SKU khusus variant
    price_override REAL,                 -- null = pakai harga produk utama
    stock_quantity INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_variants_product ON product_variants(product_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_variants_unique ON product_variants(product_id, variant_name, variant_value);
