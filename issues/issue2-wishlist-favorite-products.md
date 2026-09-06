# Issue 2: Wishlist / Favorite Products untuk Buyer

> **Prioritas:** 🟢 MEDIUM  
> **Estimasi:** 2-3 hari  
> **Kesulitan:** ⭐⭐ Mudah  
> **Prerequisite:** Tidak ada — bisa dikerjakan independen

---

## 📋 Deskripsi

Buyer saat ini tidak bisa menyimpan produk favorit.  
Issue ini menambahkan fitur **Wishlist** supaya buyer bisa:
- Menandai produk sebagai favorit (❤️)
- Melihat daftar wishlist di dashboard buyer
- Menghapus produk dari wishlist

---

## 🎯 Acceptance Criteria

- [ ] Buyer bisa toggle wishlist (add/remove) dari halaman produk
- [ ] Buyer bisa lihat daftar wishlist di tab "Wishlist" di dashboard buyer
- [ ] Buyer bisa "Add to Cart" langsung dari wishlist
- [ ] Guest (belum login) diminta login dulu saat klik wishlist
- [ ] Unit test minimal 2 test case baru

---

## 📐 Langkah-Langkah

### Langkah 1: Buat Migration Baru

**File:** `migrations/sqlite/013_create_wishlist.sql`

```sql
CREATE TABLE IF NOT EXISTS buyer_wishlist (
    id TEXT PRIMARY KEY NOT NULL,
    buyer_id TEXT NOT NULL,
    product_id TEXT NOT NULL REFERENCES catalog(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(buyer_id, product_id)
);

CREATE INDEX idx_wishlist_buyer ON buyer_wishlist(buyer_id);
```

### Langkah 2: Tambah DTO di Contracts

**File:** `crates/contracts/src/lib.rs`

Tambahkan struct baru (di bawah `BuyerProfile` sekitar line 1050):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WishlistItem {
    pub id: String,
    pub buyer_id: String,
    pub product_id: String,
    pub product_name: String,       // JOIN dari catalog
    pub product_price_cents: i64,   // JOIN dari catalog
    pub product_image_url: Option<String>,
    pub created_at: String,
}
```

### Langkah 3: Tambah Method di BuyerContract Trait

**File:** `crates/contracts/src/lib.rs`

Cari `pub trait BuyerContract` (sekitar line 1063), tambahkan:

```rust
    // --- Wishlist ---
    async fn get_wishlist(&self, buyer_id: &str) -> Result<Vec<WishlistItem>, ContractError>;
    async fn add_to_wishlist(&self, buyer_id: &str, product_id: &str) -> Result<WishlistItem, ContractError>;
    async fn remove_from_wishlist(&self, buyer_id: &str, product_id: &str) -> Result<(), ContractError>;
    async fn is_in_wishlist(&self, buyer_id: &str, product_id: &str) -> Result<bool, ContractError>;
```

### Langkah 4: Implementasi di BuyerModule

**File:** `crates/modules/buyer/src/lib.rs`

Contoh implementasi `add_to_wishlist`:

```rust
async fn add_to_wishlist(&self, buyer_id: &str, product_id: &str) -> Result<WishlistItem, ContractError> {
    let id = uuid::Uuid::new_v4().to_string();
    
    // Check product exists
    sqlx::query!("SELECT id FROM catalog WHERE id = ?", product_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| ContractError::DatabaseError(e.to_string()))?
        .ok_or(ContractError::NotFound("Product not found".into()))?;
    
    sqlx::query!(
        "INSERT INTO buyer_wishlist (id, buyer_id, product_id) VALUES (?, ?, ?)",
        id, buyer_id, product_id
    )
    .execute(&self.db)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint failed") {
            ContractError::AlreadyExists("Product already in wishlist".into())
        } else {
            ContractError::DatabaseError(e.to_string())
        }
    })?;
    
    // Return with product details (JOIN)
    self.get_wishlist_item(&id).await
}
```

### Langkah 5: Tambah Handler HTTP

**File:** `crates/web/src/handlers/buyer.rs`

Tambahkan handler baru:

```rust
/// GET /api/v1/buyer/wishlist
pub async fn get_wishlist_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<BuyerClaims>,
) -> Result<Json<Vec<WishlistItem>>, ApiError> {
    let items = state.buyer_contract
        .get_wishlist(&claims.sub)
        .await
        .map_err(ApiError::from_contract)?;
    Ok(Json(items))
}

/// POST /api/v1/buyer/wishlist/:product_id
pub async fn add_to_wishlist_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<BuyerClaims>,
    Path(product_id): Path<String>,
) -> Result<(StatusCode, Json<WishlistItem>), ApiError> {
    let item = state.buyer_contract
        .add_to_wishlist(&claims.sub, &product_id)
        .await
        .map_err(ApiError::from_contract)?;
    Ok((StatusCode::CREATED, Json(item)))
}

/// DELETE /api/v1/buyer/wishlist/:product_id
pub async fn remove_from_wishlist_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<BuyerClaims>,
    Path(product_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    state.buyer_contract
        .remove_from_wishlist(&claims.sub, &product_id)
        .await
        .map_err(ApiError::from_contract)?;
    Ok(StatusCode::NO_CONTENT)
}
```

### Langkah 6: Daftarkan Route

**File:** `crates/web/src/routes.rs`

Tambahkan di `buyer_routes`:

```rust
.route("/api/v1/buyer/wishlist", get(get_wishlist_handler))
.route(
    "/api/v1/buyer/wishlist/:product_id",
    post(add_to_wishlist_handler).delete(remove_from_wishlist_handler),
)
```

### Langkah 7: Update Frontend (Storefront)

**File:** `crates/web/static/store/store-catalog.js`

Tambahkan tombol ❤️ di setiap product card:

```javascript
function renderWishlistButton(productId) {
    return `<button class="wishlist-btn" onclick="toggleWishlist('${productId}')" title="Tambah ke Wishlist">
        <span class="wishlist-icon">♡</span>
    </button>`;
}

async function toggleWishlist(productId) {
    if (!window.StoreState?.buyerToken) {
        window.showStoreToast('Silakan login terlebih dahulu', 'warning');
        window.switchStoreTab('auth');
        return;
    }
    // POST or DELETE tergantung status saat ini
}
```

### Langkah 8: Tambah Tab Wishlist di Buyer Dashboard

**File:** `crates/web/static/store/store-auth.js`

Tambahkan section wishlist di dashboard buyer. Gunakan pattern yang sama dengan order history tab.

### Langkah 9: Tambah CSS Styling

**File:** `crates/web/static/store.css`

```css
.wishlist-btn {
    position: absolute;
    top: 8px;
    right: 8px;
    background: rgba(255,255,255,0.9);
    border: none;
    border-radius: 50%;
    width: 36px;
    height: 36px;
    cursor: pointer;
    transition: all 0.2s;
}
.wishlist-btn.active .wishlist-icon { color: #e74c3c; }
.wishlist-btn:hover { transform: scale(1.1); }
```

### Langkah 10: Tulis Unit Test

**File:** `crates/web/tests/wishlist_test.rs`

```rust
#[tokio::test]
async fn test_add_and_list_wishlist() {
    // 1. Login buyer
    // 2. Add 2 products to wishlist
    // 3. List wishlist → expect 2 items
    // 4. Remove 1 → expect 1 item
}

#[tokio::test]
async fn test_duplicate_wishlist_returns_error() {
    // 1. Add product to wishlist
    // 2. Add same product again → expect 409 Conflict or appropriate error
}
```

### Langkah 11: Verifikasi

```bash
cargo test --workspace
cargo check --workspace
```

---

## ⚠️ Perhatian

- Wishlist endpoint HARUS dilindungi `require_buyer_auth` middleware
- Gunakan `UNIQUE(buyer_id, product_id)` constraint untuk hindari duplikat
- Saat produk dihapus dari catalog, wishlist item harus ikut terhapus (CASCADE)
- Tampilkan jumlah item di wishlist badge di navbar storefront
