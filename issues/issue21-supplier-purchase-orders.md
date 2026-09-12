# Issue 21: Supplier Management & Automated Restocking Purchase Orders (PO) (Phase 4 — #4)

> **Prioritas:** 🟡 HIGH  
> **Estimasi:** 3-4 hari  
> **Kesulitan:** ⭐⭐⭐ Menengah  
> **Prerequisite:** Selesai Issue #38 (Inventory & Catalog)  

---

## 📋 Deskripsi

Dalam operasional bisnis ritel, stok tidak hanya berkurang karena penjualan, tetapi harus dipasok kembali (*restocking*) dari pemasok / supplier.  
Di file antarmuka `index.html`, fitur ini telah disiapkan sebagai: *"Fitur Pembelian & Supplier PO siap digunakan untuk restocking otomatis."*

Issue ini mengimplementasikan:
1. **Manajemen Data Supplier**: Nama pemasok, kontak person, nomor telepon/WhatsApp, email, dan alamat gudang supplier.
2. **Purchase Order (PO)**: Pembuatan dokumen pemesanan barang ke supplier dengan nomor PO unik (`PO-YYYYMMDD-XXXX`), status alur kerja (`draft` ➔ `ordered` ➔ `received` ➔ `cancelled`), estimasi tanggal tiba, dan rincian harga beli pokok (*cost per unit*).
3. **Penerimaan Barang (Goods Receiving) & Auto-Restock**: Ketika status PO diubah menjadi `received`, sistem secara otomatis menambah stok produk/varian terkait di `InventoryModule` dan mencatat mutasinya di `stock_adjustment_logs`.

---

## 🎯 Acceptance Criteria

- [ ] Admin dapat melakukan CRUD data Supplier di Admin Hub.
- [ ] Admin dapat membuat dokumen Purchase Order (PO) dengan memilih supplier dan menambahkan daftar produk/varian beserta jumlah kuantitas dan harga beli satuan.
- [ ] Status PO mengikuti siklus: `Draft` ➔ `Ordered` (Terkirim ke Supplier) ➔ `Received` (Barang Tiba) atau `Cancelled`.
- [ ] Ketika PO diubah ke `Received`, kuantitas stok di `inventory` otomatis bertambah sesuai jumlah barang PO tanpa perlu input manual satu per satu.
- [ ] Mutasi stok otomatis tercatat di tabel `stock_adjustment_logs` dengan referensi `reference_id = PO number` dan `reason = "Penerimaan PO Supplier"`.
- [ ] Minimal 3 unit test untuk alur pembuatan PO, validasi status transition, dan verifikasi mutasi stok saat barang diterima.

---

## 📐 Langkah-Langkah Pengerjaan

### Langkah 1: Migrasi Database
**File:** `migrations/sqlite/024_create_suppliers_and_po.sql`

```sql
CREATE TABLE IF NOT EXISTS suppliers (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    contact_person TEXT,
    phone TEXT,
    email TEXT,
    address TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS purchase_orders (
    id TEXT PRIMARY KEY NOT NULL,
    po_number TEXT NOT NULL UNIQUE,
    supplier_id TEXT NOT NULL REFERENCES suppliers(id),
    status TEXT NOT NULL DEFAULT 'draft',    -- draft, ordered, received, cancelled
    total_cost_cents INTEGER NOT NULL DEFAULT 0,
    expected_delivery_date TEXT,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS purchase_order_items (
    id TEXT PRIMARY KEY NOT NULL,
    po_id TEXT NOT NULL REFERENCES purchase_orders(id) ON DELETE CASCADE,
    product_id TEXT NOT NULL REFERENCES catalog(id),
    variant_id TEXT REFERENCES product_variants(id),
    quantity INTEGER NOT NULL,
    unit_cost_cents INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_po_supplier ON purchase_orders(supplier_id);
CREATE INDEX idx_po_status ON purchase_orders(status);
```

### Langkah 2: Kontrak Interface & DTO
**File:** `crates/contracts/src/lib.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SupplierDto {
    pub id: String,
    pub name: String,
    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PurchaseOrderDto {
    pub id: String,
    pub po_number: String,
    pub supplier_name: String,
    pub status: String,
    pub total_cost_cents: i64,
    pub items: Vec<PurchaseOrderItemDto>,
    pub created_at: String,
}
```

### Langkah 3: Integrasi Inventory Restock Handler
**File:** `crates/web/src/handlers/supplier.rs` & `crates/modules/inventory/src/lib.rs`
- Saat endpoint `PUT /admin/purchase-orders/:id/receive` dipanggil:
  1. Validasi bahwa status PO saat ini adalah `ordered`.
  2. Buka transaksi database SQLx.
  3. Untuk setiap item PO, panggil method internal `InventoryModule::adjust_stock(product_id, quantity, StockAdjustmentReason::SupplierRestock)`.
  4. Ubah status PO menjadi `received` dan commit transaksi.

### Langkah 4: Tampilan Frontend Admin Hub
**File:** `frontend/src/lib/AdminHub.svelte`
- Tambahkan tab baru di navigasi Admin Hub: **"🏢 Supplier & Pembelian (PO)"**.
- Formulir input PO baru dengan pemilihan supplier, pemilihan produk dari catalog, dan kalkulasi total biaya pengadaan.
- Tombol aksi **"Konfirmasi Barang Diterima (Restock)"** dengan modal konfirmasi jumlah barang yang datang.

### Langkah 5: Unit & Integration Tests
**File:** `crates/web/tests/supplier_po_test.rs`
- Uji CRUD data Supplier.
- Uji pembuatan PO baru.
- Uji perubahan status PO ke `received` dan verifikasi bahwa stok produk di catalog bertambah secara akurat.
