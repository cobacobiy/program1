-- Migration: 018_create_categories.sql
-- Dedicated categories table with product relationships and sort ordering

CREATE TABLE IF NOT EXISTS categories (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    slug TEXT NOT NULL UNIQUE,
    description TEXT,
    icon TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_categories_slug ON categories(slug);
CREATE INDEX IF NOT EXISTS idx_categories_sort_order ON categories(sort_order);

-- Add category_id foreign key column to catalog_items
ALTER TABLE catalog_items ADD COLUMN category_id TEXT REFERENCES categories(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_catalog_items_category_id ON catalog_items(category_id);

-- Seed default categories
INSERT OR IGNORE INTO categories (id, name, slug, description, icon, sort_order, is_active) VALUES
    ('cat-001', 'Semua Produk', 'semua', 'Semua koleksi produk toko', '🛍️', 0, 1),
    ('cat-002', 'Peripherals', 'peripherals', 'Keyboard, mouse, dan perangkat input komputer', '⌨️', 1, 1),
    ('cat-003', 'Accessories', 'accessories', 'Aksesoris meja kerja dan perlengkapan audio/monitor', '🎧', 2, 1),
    ('cat-004', 'Elektronik', 'elektronik', 'Gawai dan perangkat elektronik modern', '📱', 3, 1),
    ('cat-005', 'Fashion', 'fashion', 'Pakaian, apparel, dan fashion terkini', '👕', 4, 1),
    ('cat-006', 'Kesehatan', 'kesehatan', 'Produk kesehatan dan suplemen harian', '💊', 5, 1),
    ('cat-007', 'Makanan & Minuman', 'makanan-minuman', 'Kuliner, camilan, dan minuman segar', '🍜', 6, 1);

-- Link existing catalog items to their categories if applicable
UPDATE catalog_items SET category_id = 'cat-002' WHERE category = 'Peripherals' AND category_id IS NULL;
UPDATE catalog_items SET category_id = 'cat-003' WHERE category = 'Accessories' AND category_id IS NULL;
