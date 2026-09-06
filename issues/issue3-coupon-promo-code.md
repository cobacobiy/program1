# Issue 3: Kupon Diskon & Promo Code

> **Prioritas:** 🔴 CRITICAL  
> **Estimasi:** 3-4 hari  
> **Kesulitan:** ⭐⭐⭐ Menengah  
> **Prerequisite:** Tidak ada — bisa dikerjakan independen

---

## 📋 Deskripsi

Toko online tanpa fitur kupon diskon kurang kompetitif.  
Issue ini menambahkan sistem **Promo Code / Coupon** yang:
- Admin bisa membuat, mengedit, dan menonaktifkan kupon
- Buyer bisa memasukkan kode promo saat checkout
- Sistem memvalidasi dan menghitung potongan harga

---

## 🎯 Acceptance Criteria

- [ ] Admin bisa CRUD kupon di dashboard (nama, kode, tipe diskon, minimum belanja, batas penggunaan, tanggal berlaku)
- [ ] Buyer bisa input promo code di halaman checkout
- [ ] Sistem validasi: kupon valid, belum kadaluarsa, belum mencapai batas penggunaan
- [ ] 2 tipe diskon: persentase (%) dan nominal tetap (Rp)
- [ ] Order record menyimpan kupon yang dipakai + total potongan
- [ ] Unit test minimal 4 test case

---

## 📐 Langkah-Langkah

### Langkah 1: Buat Migration Baru

**File:** `migrations/sqlite/013_create_coupons.sql`

```sql
CREATE TABLE IF NOT EXISTS coupons (
    id TEXT PRIMARY KEY NOT NULL,
    code TEXT NOT NULL UNIQUE,                -- e.g. "DISKON20"
    description TEXT,
    discount_type TEXT NOT NULL DEFAULT 'percentage', -- 'percentage' or 'fixed'
    discount_value INTEGER NOT NULL,          -- cents untuk fixed, basis points untuk persen (2000 = 20%)
    min_order_cents INTEGER NOT NULL DEFAULT 0,  -- minimum order value
    max_discount_cents INTEGER,               -- cap diskon (untuk percentage)
    usage_limit INTEGER,                      -- NULL = unlimited
    usage_count INTEGER NOT NULL DEFAULT 0,
    valid_from TEXT NOT NULL,
    valid_until TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS coupon_usages (
    id TEXT PRIMARY KEY NOT NULL,
    coupon_id TEXT NOT NULL REFERENCES coupons(id),
    buyer_id TEXT NOT NULL,
    order_id TEXT NOT NULL,
    discount_amount_cents INTEGER NOT NULL,
    used_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(coupon_id, buyer_id, order_id)
);

CREATE INDEX idx_coupons_code ON coupons(code);
CREATE INDEX idx_coupon_usages_buyer ON coupon_usages(buyer_id);
```

### Langkah 2: Tambah DTO di Contracts

**File:** `crates/contracts/src/lib.rs`

```rust
// ===== Coupon DTOs =====

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum DiscountType {
    #[serde(rename = "percentage")]
    Percentage,
    #[serde(rename = "fixed")]
    Fixed,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Coupon {
    pub id: String,
    pub code: String,
    pub description: Option<String>,
    pub discount_type: DiscountType,
    pub discount_value: i64,       // basis points (2000=20%) or cents
    pub min_order_cents: i64,
    pub max_discount_cents: Option<i64>,
    pub usage_limit: Option<i32>,
    pub usage_count: i32,
    pub valid_from: String,
    pub valid_until: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateCouponRequest {
    #[validate(length(min = 3, max = 30))]
    pub code: String,
    pub description: Option<String>,
    pub discount_type: DiscountType,
    #[validate(range(min = 1))]
    pub discount_value: i64,
    pub min_order_cents: Option<i64>,
    pub max_discount_cents: Option<i64>,
    pub usage_limit: Option<i32>,
    pub valid_from: String,
    pub valid_until: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CouponValidation {
    pub is_valid: bool,
    pub coupon: Option<Coupon>,
    pub discount_amount_cents: i64,
    pub error_message: Option<String>,
}
```

### Langkah 3: Buat CouponContract Trait Baru

**File:** `crates/contracts/src/lib.rs`

```rust
#[async_trait]
pub trait CouponContract: Send + Sync {
    /// Admin: List all coupons
    async fn list_coupons(&self) -> Result<Vec<Coupon>, ContractError>;
    /// Admin: Create a new coupon
    async fn create_coupon(&self, req: CreateCouponRequest) -> Result<Coupon, ContractError>;
    /// Admin: Update coupon
    async fn update_coupon(&self, id: &str, req: CreateCouponRequest) -> Result<Coupon, ContractError>;
    /// Admin: Deactivate coupon
    async fn deactivate_coupon(&self, id: &str) -> Result<(), ContractError>;
    /// Buyer: Validate promo code against order subtotal
    async fn validate_coupon(&self, code: &str, subtotal_cents: i64, buyer_id: &str) -> Result<CouponValidation, ContractError>;
    /// Internal: Record coupon usage after order created
    async fn record_usage(&self, coupon_id: &str, buyer_id: &str, order_id: &str, discount_cents: i64) -> Result<(), ContractError>;
}
```

### Langkah 4: Buat Module Baru `crates/modules/coupon/`

Buat folder dan file baru:

```
crates/modules/coupon/
├── Cargo.toml
└── src/
    └── lib.rs
```

**File:** `crates/modules/coupon/Cargo.toml`

```toml
[package]
name = "program1-module-coupon"
version.workspace = true
edition.workspace = true

[dependencies]
program1-contracts = { path = "../../contracts" }
sqlx.workspace = true
async-trait.workspace = true
uuid.workspace = true
chrono.workspace = true
tracing.workspace = true
```

**File:** `crates/modules/coupon/src/lib.rs`

Implementasikan `CouponContract` trait. Logika validasi kupon:

```rust
async fn validate_coupon(&self, code: &str, subtotal_cents: i64, buyer_id: &str) -> Result<CouponValidation, ContractError> {
    let coupon = sqlx::query_as!(/* ... */)
        .fetch_optional(&self.db).await?;

    let coupon = match coupon {
        Some(c) => c,
        None => return Ok(CouponValidation {
            is_valid: false,
            coupon: None,
            discount_amount_cents: 0,
            error_message: Some("Kode promo tidak ditemukan".into()),
        }),
    };

    // Check: is_active, valid_from <= now <= valid_until, usage_count < usage_limit
    // Check: subtotal >= min_order_cents
    // Calculate discount based on type (percentage vs fixed)
    // Apply max_discount_cents cap
    
    Ok(CouponValidation { /* ... */ })
}
```

### Langkah 5: Register di Workspace

**File:** `Cargo.toml` (root workspace)

Tambahkan di `members`:

```toml
members = [
    # ... existing members ...
    "crates/modules/coupon",
]
```

### Langkah 6: Tambah ke AppState

**File:** `crates/web/src/state.rs`

```rust
pub coupon_contract: Arc<dyn CouponContract>,
```

**File:** `crates/web/src/main.rs`

```rust
let coupon_module = Arc::new(CouponModule::new(db_pool.clone()));
// ...tambahkan ke AppState
```

### Langkah 7: Tambah Handler HTTP

**File:** `crates/web/src/handlers/coupon.rs` (file baru)

- `list_coupons_handler` (admin)
- `create_coupon_handler` (admin)
- `validate_coupon_handler` (buyer) — POST `/api/v1/buyer/coupon/validate`
- `deactivate_coupon_handler` (admin)

### Langkah 8: Daftarkan Routes

**File:** `crates/web/src/routes.rs`

Admin routes:
```rust
.route("/api/v1/admin/coupons", get(list_coupons_handler).post(create_coupon_handler))
.route("/api/v1/admin/coupons/:id/deactivate", post(deactivate_coupon_handler))
```

Buyer routes:
```rust
.route("/api/v1/buyer/coupon/validate", post(validate_coupon_handler))
```

### Langkah 9: Update Checkout Flow

**File:** `crates/web/static/store/store-checkout.js`

Tambahkan input field promo code di checkout form:

```javascript
function renderCouponInput() {
    return `
    <div class="coupon-section">
        <input type="text" id="couponCode" placeholder="Masukkan kode promo" />
        <button onclick="applyCoupon()" class="btn-apply-coupon">Terapkan</button>
        <div id="couponResult"></div>
    </div>`;
}

async function applyCoupon() {
    const code = document.getElementById('couponCode').value.trim();
    const res = await fetch('/api/v1/buyer/coupon/validate', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${window.StoreState.buyerToken}`
        },
        body: JSON.stringify({ code, subtotal_cents: calculateSubtotal() })
    });
    const data = await res.json();
    // Show discount or error message
}
```

### Langkah 10: Update Admin Dashboard

**File:** `crates/web/static/admin/admin-catalog.js` atau buat file baru `admin-coupons.js`

Buat halaman manajemen kupon dengan tabel CRUD.

### Langkah 11: Tulis Unit Tests

**File:** `crates/web/tests/coupon_test.rs`

```rust
#[tokio::test] async fn test_create_and_validate_percentage_coupon() { /* ... */ }
#[tokio::test] async fn test_create_and_validate_fixed_coupon() { /* ... */ }
#[tokio::test] async fn test_expired_coupon_rejected() { /* ... */ }
#[tokio::test] async fn test_min_order_not_met() { /* ... */ }
#[tokio::test] async fn test_usage_limit_exceeded() { /* ... */ }
```

### Langkah 12: Verifikasi

```bash
cargo test --workspace
cargo check --workspace
```

---

## ⚠️ Perhatian

- Kode kupon harus **case-insensitive** (simpan UPPERCASE di DB)
- Kalkulasi diskon harus akurat — gunakan integer (cents), BUKAN floating point
- Jika `discount_type = percentage`, `discount_value` dalam basis points (2000 = 20%)
- Selalu cek `usage_limit` dan `usage_count` secara atomic (hindari race condition)
- Tambahkan `CouponModule` ke workspace `Cargo.toml` `members` list
