# Issue 1: Tambah Product Variant (Ukuran, Warna, dll.)

> **Prioritas:** 🟡 HIGH  
> **Estimasi:** 3-4 hari  
> **Kesulitan:** ⭐⭐⭐ Menengah  
> **Prerequisite:** Tidak ada — bisa dikerjakan independen

---

## 📋 Deskripsi

Saat ini, setiap produk di catalog hanya punya 1 harga dan 1 stok.  
Di dunia e-commerce nyata, produk sering punya **variant** (misal: ukuran S/M/L/XL, warna Merah/Biru, dsb.).  
Issue ini menambahkan fitur Product Variant ke module Catalog + Inventory.

---

## 🎯 Acceptance Criteria

- [ ] Buyer bisa lihat variant di halaman produk (e.g. "Ukuran: S, M, L")
- [ ] Setiap variant punya harga & stok sendiri
- [ ] Admin bisa CRUD variant lewat dashboard
- [ ] Checkout menggunakan `variant_id` bukan `product_id`
- [ ] Unit test minimal 3 test case baru

---

## 📐 Langkah-Langkah

### Langkah 1: Tambah Migration Baru (Database)

**File:** `migrations/sqlite/013_create_product_variants.sql`

```sql
CREATE TABLE IF NOT EXISTS product_variants (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES catalog(id) ON DELETE CASCADE,
    variant_name TEXT NOT NULL,          -- e.g. "Ukuran"
    variant_value TEXT NOT NULL,         -- e.g. "XL"
    sku TEXT,                            -- optional SKU khusus variant
    price_override_cents INTEGER,        -- null = pakai harga produk utama
    stock_quantity INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_variants_product ON product_variants(product_id);
CREATE UNIQUE INDEX idx_variants_unique ON product_variants(product_id, variant_name, variant_value);
```

### Langkah 2: Tambah DTO di Contracts

**File:** `crates/contracts/src/lib.rs`

Cari struct `CatalogItem` (sekitar line 370-390), lalu tambahkan struct baru di bawahnya:

```rust
/// Represents a product variant (size, color, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProductVariant {
    pub id: String,
    pub product_id: String,
    pub variant_name: String,
    pub variant_value: String,
    pub sku: Option<String>,
    pub price_override_cents: Option<i64>,
    pub stock_quantity: i32,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateVariantRequest {
    #[validate(length(min = 1, max = 50))]
    pub variant_name: String,
    #[validate(length(min = 1, max = 100))]
    pub variant_value: String,
    pub sku: Option<String>,
    pub price_override_cents: Option<i64>,
    pub stock_quantity: i32,
}
```

### Langkah 3: Tambah Method di CatalogContract Trait

**File:** `crates/contracts/src/lib.rs`

Cari `pub trait CatalogContract` (sekitar line 400), tambahkan method baru:

```rust
    // --- Variant Management ---
    async fn list_variants(&self, product_id: &str) -> Result<Vec<ProductVariant>, ContractError>;
    async fn create_variant(&self, product_id: &str, req: CreateVariantRequest) -> Result<ProductVariant, ContractError>;
    async fn update_variant(&self, variant_id: &str, req: CreateVariantRequest) -> Result<ProductVariant, ContractError>;
    async fn delete_variant(&self, variant_id: &str) -> Result<(), ContractError>;
```

### Langkah 4: Implementasi di CatalogModule

**File:** `crates/modules/catalog/src/lib.rs`

Tambahkan implementasi untuk setiap method baru di atas. Contoh untuk `list_variants`:

```rust
async fn list_variants(&self, product_id: &str) -> Result<Vec<ProductVariant>, ContractError> {
    let rows = sqlx::query_as!(
        ProductVariant,
        r#"SELECT id, product_id, variant_name, variant_value, sku,
                  price_override_cents, stock_quantity,
                  is_active as "is_active: bool",
                  created_at, updated_at
           FROM product_variants
           WHERE product_id = ? AND is_active = 1
           ORDER BY variant_name, variant_value"#,
        product_id
    )
    .fetch_all(&self.db)
    .await
    .map_err(|e| ContractError::DatabaseError(e.to_string()))?;

    Ok(rows)
}
```

### Langkah 5: Tambah Handler HTTP

**File:** `crates/web/src/handlers/catalog.rs`

Tambahkan handler baru:

```rust
pub async fn list_variants_handler(
    State(state): State<AppState>,
    Path(product_id): Path<String>,
) -> Result<Json<Vec<ProductVariant>>, ApiError> {
    let variants = state.catalog_contract
        .list_variants(&product_id)
        .await
        .map_err(ApiError::from_contract)?;
    Ok(Json(variants))
}

pub async fn create_variant_handler(
    State(state): State<AppState>,
    Path(product_id): Path<String>,
    ValidatedJson(req): ValidatedJson<CreateVariantRequest>,
) -> Result<(StatusCode, Json<ProductVariant>), ApiError> {
    let variant = state.catalog_contract
        .create_variant(&product_id, req)
        .await
        .map_err(ApiError::from_contract)?;
    Ok((StatusCode::CREATED, Json(variant)))
}
```

### Langkah 6: Daftarkan Route Baru

**File:** `crates/web/src/routes.rs`

Tambahkan di `protected_routes` (seller access):

```rust
.route("/api/v1/catalog/:id/variants", get(list_variants_handler).post(create_variant_handler))
.route("/api/v1/catalog/:id/variants/:vid", put(update_variant_handler).delete(delete_variant_handler))
```

Dan di `public_routes` (buyer bisa lihat variant):

```rust
.route("/api/v1/catalog/:id/variants", get(list_variants_handler))
```

### Langkah 7: Update Frontend (Admin Dashboard)

**File:** `crates/web/static/admin/admin-catalog.js`

Tambahkan UI section untuk manage variant di product detail. Gunakan pattern yang sama dengan inventory management.

### Langkah 8: Update Frontend (Storefront)

**File:** `crates/web/static/store/store-catalog.js`

Saat buyer klik produk, fetch variant list dan tampilkan sebagai dropdown/radio button sebelum "Add to Cart".

### Langkah 9: Tulis Unit Tests

**File:** `crates/web/tests/variant_test.rs`

```rust
#[tokio::test]
async fn test_create_and_list_variants() {
    // 1. Create a product
    // 2. Create 3 variants (S, M, L)
    // 3. List variants → harus dapat 3
    // 4. Delete 1 variant → list → harus dapat 2
}

#[tokio::test]
async fn test_variant_price_override() {
    // 1. Create product with price 100_000
    // 2. Create variant with price_override = 120_000
    // 3. Verify response contains correct override price
}

#[tokio::test]
async fn test_duplicate_variant_rejected() {
    // 1. Create variant "Ukuran" = "XL"
    // 2. Try create duplicate → expect error
}
```

### Langkah 10: Verifikasi

```bash
cargo test --workspace
cargo check --workspace
```

---

## ⚠️ Perhatian

- **JANGAN** mengubah data atau interface yang sudah ada (backward compatible)
- Pastikan `product_variants` table punya foreign key ke `catalog(id)` dengan `ON DELETE CASCADE`
- Jika `price_override_cents` = `NULL`, gunakan harga produk utama
- Ikuti naming convention yang sudah ada: snake_case untuk Rust, camelCase untuk JavaScript
