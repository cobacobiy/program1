# Issue 7: Product Category & Filtering

> **Prioritas:** 🟡 HIGH  
> **Estimasi:** 2-3 hari  
> **Kesulitan:** ⭐⭐ Mudah  
> **Prerequisite:** Catalog module sudah ada (sudah DONE)

---

## 📋 Deskripsi

Saat ini produk di catalog tidak punya **kategori**.  
Jika produk sudah banyak (>50), buyer kesulitan menemukan produk yang dicari.  
Issue ini menambahkan sistem kategori untuk organisasi produk yang lebih baik.

---

## 🎯 Acceptance Criteria

- [x] Admin bisa CRUD kategori (nama, deskripsi, icon/emoji)
- [x] Admin bisa assign produk ke kategori saat create/edit produk
- [x] Storefront menampilkan filter by kategori (sidebar atau tabs)
- [x] API catalog support query parameter `?category=electronics`
- [x] 1 produk bisa punya 1 kategori (simpel) atau banyak (many-to-many)
- [x] Unit test minimal 2 test case

---

## 📐 Langkah-Langkah

### Langkah 1: Buat Migration

**File:** `migrations/sqlite/013_create_categories.sql`

```sql
CREATE TABLE IF NOT EXISTS categories (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    slug TEXT NOT NULL UNIQUE,         -- URL-friendly: "elektronik", "fashion-wanita"
    description TEXT,
    icon TEXT,                          -- emoji or icon class
    sort_order INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Tambah column category_id ke catalog
ALTER TABLE catalog ADD COLUMN category_id TEXT REFERENCES categories(id);

CREATE INDEX idx_catalog_category ON catalog(category_id);

-- Seed default categories
INSERT OR IGNORE INTO categories (id, name, slug, icon, sort_order) VALUES
    ('cat-001', 'Semua Produk', 'semua', '🛍️', 0),
    ('cat-002', 'Elektronik', 'elektronik', '📱', 1),
    ('cat-003', 'Fashion', 'fashion', '👕', 2),
    ('cat-004', 'Makanan & Minuman', 'makanan-minuman', '🍜', 3),
    ('cat-005', 'Kesehatan', 'kesehatan', '💊', 4);
```

### Langkah 2: Tambah DTO di Contracts

**File:** `crates/contracts/src/lib.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub product_count: i64,      // computed field
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateCategoryRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(length(min = 1, max = 100))]
    pub slug: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub sort_order: Option<i32>,
}
```

### Langkah 3: Tambah Method di CatalogContract

**File:** `crates/contracts/src/lib.rs`

```rust
// Di pub trait CatalogContract:
async fn list_categories(&self) -> Result<Vec<Category>, ContractError>;
async fn create_category(&self, req: CreateCategoryRequest) -> Result<Category, ContractError>;
async fn update_category(&self, id: &str, req: CreateCategoryRequest) -> Result<Category, ContractError>;
async fn delete_category(&self, id: &str) -> Result<(), ContractError>;
```

### Langkah 4: Update CatalogItem DTO

**File:** `crates/contracts/src/lib.rs`

Tambahkan field `category_id` dan `category_name` ke `CatalogItem`:

```rust
pub struct CatalogItem {
    // ... existing fields ...
    pub category_id: Option<String>,
    pub category_name: Option<String>,  // from JOIN
}
```

### Langkah 5: Update List Catalog Query

**File:** `crates/modules/catalog/src/lib.rs`

Update query `list_catalog` untuk support filter category:

```rust
// Tambah parameter category_id ke method signature
async fn list_catalog(&self, params: CatalogQueryParams) -> Result<CatalogPage, ContractError> {
    let mut query = String::from("SELECT c.*, cat.name as category_name FROM catalog c LEFT JOIN categories cat ON c.category_id = cat.id WHERE 1=1");
    
    if let Some(cat_id) = &params.category_id {
        query.push_str(&format!(" AND c.category_id = '{}'", cat_id));
    }
    
    // ... existing search, pagination logic ...
}
```

### Langkah 6: Implementasi CRUD Category

**File:** `crates/modules/catalog/src/lib.rs`

```rust
async fn list_categories(&self) -> Result<Vec<Category>, ContractError> {
    let rows = sqlx::query_as!(
        CategoryRow,
        r#"SELECT c.*, 
                  (SELECT COUNT(*) FROM catalog p WHERE p.category_id = c.id) as product_count
           FROM categories c 
           WHERE c.is_active = 1
           ORDER BY c.sort_order, c.name"#
    )
    .fetch_all(&self.db)
    .await
    .map_err(|e| ContractError::DatabaseError(e.to_string()))?;
    
    Ok(rows.into_iter().map(|r| r.into()).collect())
}
```

### Langkah 7: Tambah Handler & Routes

**File:** `crates/web/src/handlers/catalog.rs`

```rust
pub async fn list_categories_handler(/* ... */) { /* ... */ }
pub async fn create_category_handler(/* ... */) { /* ... */ }
```

**File:** `crates/web/src/routes.rs`

Public:
```rust
.route("/api/v1/categories", get(list_categories_handler))
```

Admin:
```rust
.route("/api/v1/admin/categories", post(create_category_handler))
.route("/api/v1/admin/categories/:id", put(update_category_handler).delete(delete_category_handler))
```

### Langkah 8: Update Storefront UI

**File:** `crates/web/static/store/store-catalog.js`

Tambahkan category filter tabs/sidebar:

```javascript
async function loadCategories() {
    const res = await fetch('/api/v1/categories');
    const categories = await res.json();
    
    const container = document.getElementById('categoryFilter');
    container.innerHTML = categories.map(cat => `
        <button class="category-btn ${cat.id === window.StoreState.selectedCategory ? 'active' : ''}"
                onclick="filterByCategory('${cat.id}')">
            ${cat.icon || '📦'} ${cat.name}
            <span class="cat-count">${cat.product_count}</span>
        </button>
    `).join('');
}

function filterByCategory(categoryId) {
    window.StoreState.selectedCategory = categoryId;
    loadCatalog(); // reload with filter
}
```

### Langkah 9: Update Admin Catalog Management

**File:** `crates/web/static/admin/admin-catalog.js`

Tambahkan dropdown kategori di form create/edit product.

### Langkah 10: Tulis Unit Test

**File:** `crates/web/tests/category_test.rs`

```rust
#[tokio::test]
async fn test_create_and_list_categories() {
    // 1. Create 3 categories
    // 2. List → expect 3 + default seeded categories
}

#[tokio::test]
async fn test_filter_catalog_by_category() {
    // 1. Create category "Elektronik"
    // 2. Create product with category_id
    // 3. Query catalog with ?category_id=... → expect only matching products
}
```

### Langkah 11: Verifikasi

```bash
cargo test --workspace
cargo check --workspace
```

---

## ⚠️ Perhatian

- Slug harus URL-safe (lowercase, hyphens, no spaces): "fashion-wanita" bukan "Fashion Wanita"
- Jangan hapus kategori yang masih punya produk — return error "Category still has N products"
- `category_id` di catalog table bisa NULL (produk tanpa kategori tetap ditampilkan)
- Seed default categories di migration, bukan di runtime code
- Filter "Semua Produk" = tidak pakai filter category (show semua)
