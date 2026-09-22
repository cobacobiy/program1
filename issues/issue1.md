# Issue #1: CORS Tidak Mengizinkan HTTP Method DELETE

## Severity: 🔴 HIGH (Bug Produksi)

## Deskripsi
CORS layer di `middleware.rs` hanya mengizinkan method `GET, POST, PUT, PATCH, OPTIONS`, tetapi **`DELETE` tidak termasuk**. Ini menyebabkan semua request DELETE dari frontend browser (seperti hapus coupon, hapus wishlist, hapus variant, hapus supplier, dll.) akan **gagal dengan CORS error** di production.

## File yang Terdampak
- `crates/web/src/middleware.rs` — fungsi `build_cors_layer()` baris 453-461

## Bukti Masalah
Lihat konfigurasi saat ini di `middleware.rs` baris 455-461:
```rust
.allow_methods([
    Method::GET,
    Method::POST,
    Method::PUT,
    Method::PATCH,
    Method::OPTIONS,
])
// ❌ Method::DELETE tidak ada!
```

Sementara beberapa route menggunakan `delete()`:
- `/api/v1/buyer/wishlist/:product_id` → `delete(remove_from_wishlist_handler)`
- `/api/v1/admin/coupons/:id` → `delete(delete_coupon_handler)`
- `/api/v1/catalog/:id/variants/:vid` → `delete(delete_variant_handler)`
- `/api/v1/buyer/addresses/:id` → `delete(delete_address_handler)`
- `/api/v1/admin/suppliers/:id` → `delete(delete_supplier)`

## Langkah Perbaikan

### Step 1: Tambahkan `Method::DELETE` ke CORS allow_methods
Buka file: `crates/web/src/middleware.rs`

Cari blok kode ini (sekitar baris 455):
```rust
.allow_methods([
    Method::GET,
    Method::POST,
    Method::PUT,
    Method::PATCH,
    Method::OPTIONS,
])
```

Ubah menjadi:
```rust
.allow_methods([
    Method::GET,
    Method::POST,
    Method::PUT,
    Method::PATCH,
    Method::DELETE,
    Method::OPTIONS,
])
```

### Step 2: Verifikasi
```bash
cargo check --workspace
cargo test --workspace
```

### Step 3: Test Manual (opsional)
Jalankan server lokal, buka frontend, dan coba hapus item dari wishlist atau coupon dari admin panel. Pastikan tidak ada CORS error di browser console.

## Estimasi Waktu: 5 menit
## Kompleksitas: Rendah (1 baris ditambahkan)
