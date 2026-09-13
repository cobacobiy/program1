-- Migration: 025_create_staff_permissions.sql
CREATE TABLE IF NOT EXISTS permissions (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,          -- contoh: 'orders:manage', 'catalog:write'
    description TEXT NOT NULL,
    category TEXT NOT NULL              -- 'Orders', 'Catalog', 'Inventory', 'Finance', 'Support', 'Marketing', 'Suppliers'
);

CREATE TABLE IF NOT EXISTS user_permissions (
    user_id TEXT NOT NULL REFERENCES user_accounts(id) ON DELETE CASCADE,
    permission_id TEXT NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, permission_id)
);

-- Seed daftar permission standar
INSERT OR IGNORE INTO permissions (id, name, description, category) VALUES
('p1', 'orders:manage', 'Memproses dan mengubah status pesanan pembeli', 'Orders'),
('p2', 'inventory:manage', 'Mengatur stok gudang dan mutasi inventori', 'Inventory'),
('p3', 'catalog:write', 'Menambah, mengedit, dan menghapus produk katalog', 'Catalog'),
('p4', 'chat:support', 'Membalas live chat dan tiket bantuan pembeli', 'Support'),
('p5', 'reports:view', 'Melihat laporan penjualan dan omzet keuangan', 'Finance'),
('p6', 'promotions:manage', 'Mengelola kupon diskon dan kampanye flash sale', 'Marketing'),
('p7', 'suppliers:manage', 'Mengelola data pemasok dan pesanan pembelian (PO)', 'Suppliers');
