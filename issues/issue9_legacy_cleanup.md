# Issue #9 — Legacy Cleanup & Code Hygiene

> **Prioritas**: 🟢 MEDIUM
> **Estimasi**: 1 hari
> **Depends On**: —
> **Blocks**: —

---

## 🔍 Masalah Saat Ini

### 1. Legacy `product` Module
Module `crates/modules/product/` masih ada di workspace tapi **TIDAK dipakai** oleh `crates/web`:
- `Cargo.toml` root masih list `crates/modules/product` tapi TIDAK ada `crates/modules/product` di web dependencies
- Contracts (`lib.rs`) sudah di-refactor ke `CatalogContract` + `InventoryContract`
- Product module masih punya `ProductContract`, `ProductDto`, `CreateProductRequest` yang sudah tidak dipakai
- Ini membingungkan developer baru

### 2. README Mismatch
README.md masih menunjukkan struktur lama:
- Module list masih sebut `user`, `product`, `order` — padahal sekarang ada `catalog`, `inventory`, `channel`, `analytics`
- API endpoint list sudah outdated

### 3. Contracts Mismatch
`crates/contracts/src/lib.rs` mungkin masih punya type yang dipakai oleh legacy `product` module (`ProductDto`, `ProductContract`, dll) — harus dicek dan di-remove jika sudah tidak dipakai di tempat lain.

### 4. `main.rs` Monolith
`crates/web/src/main.rs` sudah 345 baris dan makin besar. Handlers, middleware, dan router semua dalam 1 file.

---

## ✅ Acceptance Criteria

### 1. Remove atau Archive Legacy Product Module

**Opsi A (Recommended)**: Hapus `crates/modules/product/` sepenuhnya
- Remove dari `Cargo.toml` root workspace members
- Remove directory `crates/modules/product/`
- Remove related types dari `crates/contracts/src/lib.rs` (jika ada `ProductDto`, `ProductContract`, `CreateProductRequest`)

**Opsi B**: Rename ke `crates/modules/_legacy_product/` dan remove dari workspace members

### 2. Update README.md

Update bagian:
- **Repository Structure** — tambahkan `catalog`, `inventory`, `channel`, `analytics`
- **REST API Endpoints** — update ke endpoint yang sebenarnya
- **Features** — tambahkan Ginee OMS inventory, channel sync, analytics

### 3. Split `main.rs` ke Multiple Files

```
crates/web/src/
├── main.rs              ← Entry point (hanya server setup)
├── state.rs             ← AppState struct
├── errors.rs            ← ApiError, error mapping
├── middleware.rs         ← Auth middleware, security headers (Issue #2)
├── handlers/
│   ├── mod.rs
│   ├── health.rs        ← health_check, store_info
│   ├── auth.rs          ← login, register (Issue #1)
│   ├── user.rs          ← user account handlers
│   ├── catalog.rs       ← catalog handlers
│   ├── inventory.rs     ← inventory handlers
│   ├── channel.rs       ← channel sync handlers
│   ├── order.rs         ← order handlers
│   └── analytics.rs     ← analytics handlers
└── routes.rs            ← Router builder
```

### 4. Check Unused Dependencies

Jalankan:
```bash
cargo install cargo-udeps
cargo +nightly udeps --workspace
```

Dan remove unused dependencies dari `Cargo.toml`.

### 5. Add `#![deny(warnings)]` dan Clippy

Tambahkan di setiap `lib.rs` dan `main.rs`:
```rust
#![deny(warnings)]
#![deny(clippy::all)]
```

Dan jalankan:
```bash
cargo clippy --workspace -- -D warnings
```

Fix semua warnings.

---

## 🧪 Verification

```bash
# 1. Pastikan compile clean
cargo check --workspace

# 2. Pastikan semua test pass
cargo test --workspace

# 3. Pastikan tidak ada warning
cargo clippy --workspace -- -D warnings

# 4. Pastikan Docker build masih berhasil
docker compose config --quiet
```

---

## ⚠️ Peringatan

- JANGAN hapus `product` module kalau masih ada module lain yang depend padanya
- Cek dulu dengan `cargo check --workspace` setelah setiap perubahan
- README update jangan sampai menghapus mermaid diagram — update saja
- File split di `main.rs` JANGAN mengubah behavior — hanya reorganize
- Semua existing test `cargo test --workspace` harus tetap PASS
