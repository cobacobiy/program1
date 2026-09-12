# Issue 24: Granular Staff Roles & Permission Matrix (Phase 4 — #7)

> **Prioritas:** 🟡 HIGH  
> **Estimasi:** 3-4 hari  
> **Kesulitan:** ⭐⭐⭐ Menengah  
> **Prerequisite:** Selesai Issue #1 (Auth & User)  

---

## 📋 Deskripsi

Saat ini sistem RBAC Program1 hanya memiliki 3 peran sederhana: `Admin`, `Staff`, dan `Buyer`.  
Dalam operasional toko nyata, karyawan toko memiliki divisi dan tanggung jawab yang berbeda-beda:
- **Staff CS / Kasir**: Mengelola Live Chat, Status Pesanan Pembeli, dan Moderasi Review. Tidak boleh melihat laporan keuangan omzet atau mengubah harga produk di katalog.
- **Staff Gudang / Warehouse**: Mengelola Stok Inventori, Status Pengiriman Kurir, dan Penerimaan Barang PO Supplier. Tidak boleh mengakses chat pembeli atau mengubah kupon diskon.
- **Staff Marketing / Promosi**: Mengelola Kupon Diskon, Flash Sale, dan Notifikasi Broadcast.
- **Super Admin**: Akses penuh ke seluruh menu, termasuk audit logs, backup database, dan manajemen akun staf.

Issue ini menambahkan matriks izin granular (*Permission-based RBAC*) agar pemilik toko dapat mengatur kewenangan tiap staf secara aman (*principle of least privilege*).

---

## 🎯 Acceptance Criteria

- [ ] Setiap pengguna dengan peran `Staff` dapat memiliki satu atau lebih izin spesifik (`permissions`), misalnya: `catalog:write`, `orders:manage`, `inventory:manage`, `chat:support`, `reports:view`, `promotions:manage`.
- [ ] Pengguna `Admin` otomatis memiliki seluruh izin secara default (*wildcard `*`*).
- [ ] Middleware otorisasi di Axum memvalidasi izin spesifik per endpoint (misal: `require_permission("orders:manage")`).
- [ ] Di antarmuka Admin Hub, navigasi sidebar dan tombol aksi otomatis disesuaikan (disembunyikan) sesuai izin yang dimiliki staf yang sedang login.
- [ ] Super Admin dapat membuat akun staf baru dan mencentang izin yang diberikan lewat modal Admin Hub.
- [ ] Minimal 3 unit test untuk: verifikasi penolakan akses jika staf tidak memiliki izin yang dibutuhkan, penerimaan akses jika izin cocok, dan hak akses penuh admin.

---

## 📐 Langkah-Langkah Pengerjaan

### Langkah 1: Migrasi Database
**File:** `migrations/sqlite/025_create_staff_permissions.sql`

```sql
CREATE TABLE IF NOT EXISTS permissions (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,          -- contoh: 'orders:manage', 'catalog:write'
    description TEXT NOT NULL,
    category TEXT NOT NULL              -- 'Orders', 'Catalog', 'Inventory', 'Finance'
);

CREATE TABLE IF NOT EXISTS user_permissions (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
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
('p6', 'promotions:manage', 'Mengelola kupon diskon dan kampanye flash sale', 'Marketing');
```

### Langkah 2: Kontrak Interface & Token Claims
**File:** `crates/contracts/src/lib.rs` & `crates/modules/auth/src/lib.rs`

Perbarui `TokenClaims` untuk memuat daftar permission:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: String,
    pub role: String,
    pub permissions: Vec<String>,
    pub exp: usize,
}
```

### Langkah 3: Middleware Permission Extractor
**File:** `crates/web/src/middleware.rs`

Buat helper extractor atau layer:
```rust
pub fn require_permission(perm: &'static str) -> impl Fn(...) {
    // Memeriksa apakah claims.role == "admin" ATAU claims.permissions mengandung perm
}
```

### Langkah 4: Tampilan Frontend Admin Hub
**File:** `frontend/src/lib/AdminHub.svelte`
- Di menu manajemen staf, tampilkan checklist kotak centang untuk setiap kategori permission saat membuat atau mengedit akun staf.
- Di bilah navigasi kiri, sembunyikan tab (seperti Laporan Penjualan) jika staf yang login tidak memiliki izin `reports:view`.

### Langkah 5: Unit & Integration Tests
**File:** `crates/web/tests/staff_rbac_test.rs`
- Uji staf dengan hanya izin `chat:support` mencoba mengakses endpoint catalog/write (mendapat 403 Forbidden).
- Uji staf dengan izin `orders:manage` sukses memproses pesanan pembeli.
- Uji admin selalu lolos pada semua permission.
