# Issue 18: Flash Sale & Limited-Time Promotional Campaigns (Phase 4 — #1)

> **Prioritas:** 🔴 CRITICAL  
> **Estimasi:** 3-4 hari  
> **Kesulitan:** ⭐⭐⭐ Menengah  
> **Prerequisite:** Selesai Issue #38 (Catalog) & Issue #35 (Payment)  

---

## 📋 Deskripsi

Di platform e-commerce modern (Shopee, Tokopedia, Lazada), **Flash Sale** adalah fitur pendorong konversi penjualan terbesar.  
Saat ini, Program1 baru mendukung kupon diskon statis (`014_create_coupons.sql`), namun belum memiliki mesin promosi berbasis jadwal waktu (*scheduled countdown campaign*) dengan kuota stok terbatas khusus flash sale.

Fitur ini menambahkan:
1. Penjadwalan sesi Flash Sale (waktu mulai, waktu selesai, banner kampanye, status aktif).
2. Item produk flash sale dengan potongan harga khusus (`flash_price_cents`), kuota flash sale (`flash_stock_allocated`), dan penghitung penjualan (`flash_stock_sold`).
3. Countdown timer real-time (`HH:mm:ss`) di frontend Svelte dan kartu produk dengan progress bar persentase ("Terjual 65%").
4. Validasi waktu server & reservasi stok atomic agar harga flash sale otomatis berakhir saat waktu habis atau kuota habis.

---

## 🎯 Acceptance Criteria

- [ ] Admin dapat membuat, mengubah jadwal, dan menonaktifkan sesi Flash Sale lewat Admin Hub.
- [ ] Admin dapat menambahkan produk ke sesi Flash Sale dengan harga khusus dan kuota stok tertentu.
- [ ] Storefront menampilkan banner Flash Sale aktif dengan hitung mundur (*countdown timer*) yang akurat.
- [ ] Kartu produk Flash Sale menampilkan badge "FLASH SALE", harga coret, harga diskon, dan progress bar stok terjual.
- [ ] Saat checkout, sistem memverifikasi bahwa sesi Flash Sale masih berlangsung dan stok alokasi masih tersedia.
- [ ] Ketika waktu sesi berakhir atau kuota habis, checkout otomatis kembali ke harga normal produk tanpa error.
- [ ] Minimal 3 integration/unit tests mencakup: validasi waktu sesi, reservasi kuota stok flash sale, dan penolakan harga flash sale setelah sesi expired.

---

## 📐 Langkah-Langkah Pengerjaan

### Langkah 1: Migrasi Database (SQLite / Postgres)
**File:** `migrations/sqlite/021_create_flash_sales.sql`

```sql
CREATE TABLE IF NOT EXISTS flash_sale_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    start_time TEXT NOT NULL,
    end_time TEXT NOT NULL,
    banner_url TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS flash_sale_items (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL REFERENCES flash_sale_sessions(id) ON DELETE CASCADE,
    product_id TEXT NOT NULL REFERENCES catalog(id) ON DELETE CASCADE,
    flash_price_cents INTEGER NOT NULL,
    allocated_quantity INTEGER NOT NULL,
    sold_quantity INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_flash_sessions_active ON flash_sale_sessions(is_active, start_time, end_time);
CREATE UNIQUE INDEX idx_flash_item_session_product ON flash_sale_items(session_id, product_id);
```

### Langkah 2: Kontrak Interface & DTO
**File:** `crates/contracts/src/lib.rs`

Tambahkan DTO dan Trait interface:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FlashSaleSessionDto {
    pub id: String,
    pub title: String,
    pub start_time: String,
    pub end_time: String,
    pub banner_url: Option<String>,
    pub is_active: bool,
    pub items: Vec<FlashSaleItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FlashSaleItemDto {
    pub id: String,
    pub session_id: String,
    pub product_id: String,
    pub product_name: String,
    pub original_price_cents: i64,
    pub flash_price_cents: i64,
    pub allocated_quantity: i32,
    pub sold_quantity: i32,
}
```

### Langkah 3: REST API Endpoints
**File:** `crates/web/src/handlers/flash_sale.rs`
- `GET /flash-sales/active` — Endpoint publik untuk mengambil sesi flash sale yang sedang berlangsung beserta sisa detik hitung mundur dan daftar produk.
- `POST /admin/flash-sales` — Endpoint admin untuk membuat sesi baru.
- `POST /admin/flash-sales/:id/items` — Menambahkan produk ke sesi flash sale.
- `PUT /admin/flash-sales/:id/toggle` — Mengaktifkan/menonaktifkan sesi.

### Langkah 4: Tampilan Frontend Svelte 5
**File:** `frontend/src/lib/FlashSaleBanner.svelte` & `frontend/src/App.svelte`
- Buat komponen `FlashSaleBanner.svelte` dengan tampilan timer oranye menyala (*vibrant badge*).
- Gunakan Svelte 5 `$state` dan `$effect` untuk memutakhirkan countdown setiap 1 detik.
- Tampilkan progress bar persentase keterjualan produk (contoh: "🔥 Segera Habis! Terjual 80%").

### Langkah 5: Unit & Integration Tests
**File:** `crates/web/tests/flash_sale_test.rs`
- Uji pengambilan sesi aktif (`GET /flash-sales/active`).
- Uji proteksi admin untuk pembuatan sesi.
- Uji transisi status saat sesi berakhir.
