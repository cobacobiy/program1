# Issue 4: Product Review & Rating oleh Buyer

> **Prioritas:** 🟡 HIGH  
> **Estimasi:** 3-4 hari  
> **Kesulitan:** ⭐⭐⭐ Menengah  
> **Prerequisite:** Order sudah delivered (`order_status = 'delivered'`)

---

## 📋 Deskripsi

Buyer yang sudah menerima pesanan harus bisa memberikan **review dan rating** (1-5 bintang).  
Fitur ini penting untuk membangun kepercayaan buyer lain dan meningkatkan konversi.

---

## 🎯 Acceptance Criteria

- [ ] Buyer hanya bisa review produk yang sudah dikirim (order status = `delivered`)
- [ ] 1 buyer = 1 review per produk per order
- [ ] Rating 1-5 bintang + teks review (opsional)
- [ ] Halaman produk menampilkan rata-rata rating dan daftar review
- [ ] Admin bisa lihat semua review di dashboard
- [ ] Unit test minimal 3 test case

---

## 📐 Langkah-Langkah

### Langkah 1: Buat Migration

**File:** `migrations/sqlite/013_create_reviews.sql`

```sql
CREATE TABLE IF NOT EXISTS product_reviews (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES catalog(id) ON DELETE CASCADE,
    buyer_id TEXT NOT NULL,
    order_id TEXT NOT NULL,
    rating INTEGER NOT NULL CHECK (rating >= 1 AND rating <= 5),
    review_text TEXT,
    is_visible INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(product_id, buyer_id, order_id)
);

CREATE INDEX idx_reviews_product ON product_reviews(product_id);
CREATE INDEX idx_reviews_buyer ON product_reviews(buyer_id);
```

### Langkah 2: Tambah DTO di Contracts

**File:** `crates/contracts/src/lib.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProductReview {
    pub id: String,
    pub product_id: String,
    pub buyer_id: String,
    pub buyer_name: String,         // Display name dari buyer profile
    pub order_id: String,
    pub rating: i32,
    pub review_text: Option<String>,
    pub is_visible: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateReviewRequest {
    #[validate(length(min = 1))]
    pub product_id: String,
    #[validate(length(min = 1))]
    pub order_id: String,
    #[validate(range(min = 1, max = 5))]
    pub rating: i32,
    #[validate(length(max = 1000))]
    pub review_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProductRatingSummary {
    pub product_id: String,
    pub average_rating: f64,
    pub total_reviews: i32,
    pub rating_distribution: [i32; 5], // [1-star count, 2-star, 3-star, 4-star, 5-star]
}
```

### Langkah 3: Tambah Method di BuyerContract atau Buat ReviewContract

Pilih salah satu approach:

**Approach A** — Tambahkan di `BuyerContract` (lebih simple):

```rust
async fn create_review(&self, buyer_id: &str, req: CreateReviewRequest) -> Result<ProductReview, ContractError>;
async fn get_reviews_for_product(&self, product_id: &str, limit: i64, offset: i64) -> Result<Vec<ProductReview>, ContractError>;
async fn get_rating_summary(&self, product_id: &str) -> Result<ProductRatingSummary, ContractError>;
```

**Approach B** — Buat `ReviewContract` baru (lebih modular, recommended untuk project besar).

### Langkah 4: Implementasi Logic

Poin penting saat implementasi `create_review`:

```rust
async fn create_review(&self, buyer_id: &str, req: CreateReviewRequest) -> Result<ProductReview, ContractError> {
    // 1. Cek order exists dan milik buyer ini
    let order = sqlx::query!("SELECT buyer_id, status FROM orders WHERE id = ?", req.order_id)
        .fetch_optional(&self.db).await?
        .ok_or(ContractError::NotFound("Order not found".into()))?;
    
    // 2. Cek ownership
    if order.buyer_id.as_deref() != Some(buyer_id) {
        return Err(ContractError::Unauthorized("Not your order".into()));
    }
    
    // 3. Cek order sudah delivered
    if order.status != "delivered" {
        return Err(ContractError::ValidationError(
            "Can only review delivered orders".into()
        ));
    }
    
    // 4. Cek belum pernah review produk ini di order ini
    // 5. Insert review
    // 6. Return review with buyer display name
}
```

### Langkah 5: Tambah Handler HTTP

**File:** `crates/web/src/handlers/buyer.rs`

```rust
/// POST /api/v1/buyer/reviews
pub async fn create_review_handler(/* ... */) { /* ... */ }

/// GET /api/v1/catalog/:id/reviews (PUBLIC)
pub async fn get_product_reviews_handler(/* ... */) { /* ... */ }

/// GET /api/v1/catalog/:id/rating (PUBLIC)
pub async fn get_product_rating_handler(/* ... */) { /* ... */ }
```

### Langkah 6: Daftarkan Routes

**File:** `crates/web/src/routes.rs`

Public (semua orang bisa lihat review):
```rust
.route("/api/v1/catalog/:id/reviews", get(get_product_reviews_handler))
.route("/api/v1/catalog/:id/rating", get(get_product_rating_handler))
```

Buyer (butuh login):
```rust
.route("/api/v1/buyer/reviews", post(create_review_handler))
```

### Langkah 7: Update Storefront — Product Detail

**File:** `crates/web/static/store/store-catalog.js`

Saat buyer klik produk, tampilkan section review di bawah:

```javascript
async function loadProductReviews(productId) {
    const res = await fetch(`/api/v1/catalog/${productId}/reviews`);
    const reviews = await res.json();
    
    const ratingRes = await fetch(`/api/v1/catalog/${productId}/rating`);
    const rating = await ratingRes.json();
    
    return `
    <div class="review-summary">
        <div class="avg-rating">
            <span class="big-number">${rating.average_rating.toFixed(1)}</span>
            <div class="stars">${renderStars(rating.average_rating)}</div>
            <span class="total">${rating.total_reviews} ulasan</span>
        </div>
    </div>
    <div class="review-list">
        ${reviews.map(r => renderReviewCard(r)).join('')}
    </div>`;
}

function renderStars(rating) {
    let stars = '';
    for (let i = 1; i <= 5; i++) {
        stars += i <= Math.round(rating) ? '★' : '☆';
    }
    return stars;
}
```

### Langkah 8: Update Storefront — Review Form

Tampilkan form review hanya untuk order yang sudah delivered:

```javascript
function renderReviewForm(productId, orderId) {
    return `
    <div class="review-form">
        <h4>Beri Ulasan</h4>
        <div class="star-input" id="starInput">
            ${[1,2,3,4,5].map(i => 
                `<span class="star-select" data-value="${i}" onclick="selectStar(${i})">☆</span>`
            ).join('')}
        </div>
        <textarea id="reviewText" placeholder="Tulis ulasan Anda (opsional)" maxlength="1000"></textarea>
        <button onclick="submitReview('${productId}', '${orderId}')" class="btn-submit-review">Kirim Ulasan</button>
    </div>`;
}
```

### Langkah 9: Tulis Unit Tests

**File:** `crates/web/tests/review_test.rs`

```rust
#[tokio::test]
async fn test_create_review_success() {
    // 1. Create buyer, create order, set status delivered
    // 2. Submit review with rating 5
    // 3. Get product reviews → expect 1 review
}

#[tokio::test]
async fn test_review_requires_delivered_order() {
    // 1. Create order with status "pending"
    // 2. Try submit review → expect error
}

#[tokio::test]
async fn test_duplicate_review_rejected() {
    // 1. Create and submit review
    // 2. Submit same review again → expect error (UNIQUE constraint)
}

#[tokio::test]
async fn test_rating_summary_calculation() {
    // 1. Submit 3 reviews with ratings 3, 4, 5
    // 2. Get rating summary → avg = 4.0, total = 3
}
```

### Langkah 10: Verifikasi

```bash
cargo test --workspace
cargo check --workspace
```

---

## ⚠️ Perhatian

- Rating HARUS integer 1-5 (validasi di backend, jangan percaya frontend)
- Buyer hanya boleh review produk dari order **miliknya** yang sudah **delivered**
- Review public routes (GET) tidak butuh auth — semua orang bisa baca review
- Review create route (POST) WAJIB `require_buyer_auth`
- Jangan expose buyer email/phone di review response (privacy)
