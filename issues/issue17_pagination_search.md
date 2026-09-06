# Issue #17 — Catalog Pagination, Search & Filtering

> **Prioritas**: 🟡 HIGH
> **Estimasi**: 2 hari
> **Depends On**: —
> **Skill Level**: Junior Rust Developer

---

## 🔍 Masalah Saat Ini

API `GET /api/v1/catalog` mengembalikan **semua produk sekaligus** tanpa pagination. Begitu juga `GET /api/v1/orders`, `GET /api/v1/inventory`, dan `GET /api/v1/admin/buyers`.

Dengan data sedikit ini tidak masalah, tapi ketika produk sudah 1000+:
- ❌ Response besar, lambat, boros bandwidth
- ❌ Tidak ada server-side search/filter
- ❌ Tidak ada sorting
- ❌ Storefront frontend harus load semua data di browser

---

## ✅ Acceptance Criteria

### Step 1: Definisikan Pagination DTO di `crates/contracts/src/lib.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct PaginationParams {
    #[validate(range(min = 1, max = 1000))]
    pub page: Option<i64>,
    #[validate(range(min = 1, max = 100))]
    pub page_size: Option<i64>,
    pub search: Option<String>,
    pub category: Option<String>,
    pub sort_by: Option<String>,      // "name", "price", "created_at"
    pub sort_order: Option<String>,   // "asc", "desc"
}

impl PaginationParams {
    pub fn page(&self) -> i64 { self.page.unwrap_or(1) }
    pub fn page_size(&self) -> i64 { self.page_size.unwrap_or(20) }
    pub fn offset(&self) -> i64 { (self.page() - 1) * self.page_size() }
}
```

### Step 2: Update `CatalogContract`

```rust
#[async_trait]
pub trait CatalogContract: Send + Sync {
    async fn list_items(&self) -> Result<Vec<CatalogItemDto>, ContractError>;

    /// Paginated listing with search, filter, sort
    async fn list_items_paginated(
        &self,
        page: i64,
        page_size: i64,
        search: Option<&str>,
        category: Option<&str>,
        sort_by: Option<&str>,
        sort_order: Option<&str>,
    ) -> Result<PaginatedResponse<CatalogItemDto>, ContractError>;

    // ... rest unchanged
}
```

### Step 3: Implementasi SQL Query di `crates/modules/catalog/src/lib.rs`

```sql
-- Count total
SELECT COUNT(*) as total FROM catalog_items
WHERE ($search IS NULL OR name LIKE '%' || $search || '%' OR sku LIKE '%' || $search || '%')
  AND ($category IS NULL OR category = $category);

-- Fetch page
SELECT * FROM catalog_items
WHERE ($search IS NULL OR name LIKE '%' || $search || '%' OR sku LIKE '%' || $search || '%')
  AND ($category IS NULL OR category = $category)
ORDER BY {sort_by} {sort_order}
LIMIT $page_size OFFSET $offset;
```

### Step 4: Update API Endpoints

**Endpoint:** `GET /api/v1/catalog?page=1&page_size=20&search=kaos&category=clothing&sort_by=price&sort_order=asc`

**Response:**
```json
{
  "data": [...],
  "total": 150,
  "page": 1,
  "page_size": 20,
  "total_pages": 8
}
```

Terapkan pagination juga ke:

| Endpoint | Query Params |
|----------|-------------|
| `GET /api/v1/catalog` | `page`, `page_size`, `search`, `category`, `sort_by`, `sort_order` |
| `GET /api/v1/orders` | `page`, `page_size`, `status`, `sort_by` |
| `GET /api/v1/inventory` | `page`, `page_size`, `search`, `sort_by` |
| `GET /api/v1/admin/buyers` | `page`, `page_size`, `search` |

### Step 5: Frontend Updates

**Storefront (`store.js`):**
- Load produk per halaman (20 per page)
- Tambahkan "Load More" button atau pagination controls
- Server-side search menggantikan client-side filter saat ini

**Admin Panel (`app.js`):**
- Table pagination (prev/next buttons, page number)
- Tampilkan "Showing 1-20 of 150 items"

### Step 6: Unit Tests

1. Test pagination: page=1 returns first 20, page=2 returns next 20
2. Test search: search="kaos" only returns matching items
3. Test category filter: category="clothing" only returns that category
4. Test sort: sort_by=price, sort_order=asc → ascending prices
5. Test edge case: page beyond total → returns empty data, correct total
6. Test default values: no params → page=1, page_size=20

---

## 📁 File Yang Harus Dibuat/Diubah

| Action | File |
|--------|------|
| **MODIFY** | `crates/contracts/src/lib.rs` (add PaginatedResponse, PaginationParams) |
| **MODIFY** | `crates/modules/catalog/src/lib.rs` (implement paginated query) |
| **MODIFY** | `crates/modules/order/src/lib.rs` (paginated orders) |
| **MODIFY** | `crates/modules/inventory/src/lib.rs` (paginated inventory) |
| **MODIFY** | `crates/modules/buyer/src/lib.rs` (paginated buyer list) |
| **MODIFY** | `crates/web/src/handlers/catalog.rs` (accept query params) |
| **MODIFY** | `crates/web/src/handlers/order.rs` (accept query params) |
| **MODIFY** | `crates/web/src/handlers/inventory.rs` (accept query params) |
| **MODIFY** | `crates/web/src/handlers/buyer.rs` (accept query params) |
| **MODIFY** | `crates/web/static/store.js` (pagination UI) |
| **MODIFY** | `crates/web/static/app.js` (admin table pagination) |

---

## ⚠️ Catatan Penting

- **Backward compatible**: `list_items()` tetap ada (tanpa pagination) untuk internal use
- **SQL injection prevention**: Jangan concat sort_by langsung ke query string — whitelist allowed column names
- Default `page_size = 20`, max `page_size = 100`
- Pastikan `cargo test --workspace` pass sebelum push
