# Issue 5: Shipping & Ongkir Integration (RajaOngkir API)

> **Prioritas:** 🔴 CRITICAL  
> **Estimasi:** 4-5 hari  
> **Kesulitan:** ⭐⭐⭐⭐ Sulit  
> **Prerequisite:** Buyer Address sudah ada (sudah DONE)

---

## 📋 Deskripsi

Saat ini checkout tidak menghitung ongkos kirim.  
Issue ini mengintegrasikan **RajaOngkir API** (atau API ongkir lainnya) untuk:
- Menghitung biaya pengiriman berdasarkan berat produk + alamat tujuan
- Buyer memilih kurir (JNE, TIKI, POS, dll.)
- Tracking number disimpan di order record

---

## 🎯 Acceptance Criteria

- [ ] Admin bisa set alamat warehouse (asal pengiriman) di config
- [ ] Produk punya field `weight_grams` untuk kalkulasi ongkir
- [ ] Buyer saat checkout bisa lihat estimasi ongkir dari beberapa kurir
- [ ] Buyer memilih kurir → ongkir ditambahkan ke total order
- [ ] Admin bisa input resi/tracking number setelah kirim barang
- [ ] Unit test minimal 3 test case

---

## 📐 Langkah-Langkah

### Langkah 1: Tambah Field `weight_grams` ke Catalog

**File:** `migrations/sqlite/013_add_product_weight.sql`

```sql
ALTER TABLE catalog ADD COLUMN weight_grams INTEGER NOT NULL DEFAULT 500;
```

### Langkah 2: Tambah Config Shipping

**File:** `.env.example`

Tambahkan:

```env
# --- Shipping (RajaOngkir) ---
RAJAONGKIR_API_KEY=your-api-key-here
RAJAONGKIR_TYPE=starter           # starter | basic | pro
SHIPPING_ORIGIN_CITY_ID=152       # ID kota asal (misal: Jakarta Selatan = 152)
```

**File:** `crates/core/src/config.rs`

Tambahkan field baru di `AppConfig`:

```rust
pub rajaongkir_api_key: String,
pub rajaongkir_type: String,
pub shipping_origin_city_id: String,
```

### Langkah 3: Buat Shipping Module Baru

```
crates/modules/shipping/
├── Cargo.toml
└── src/
    └── lib.rs
```

**File:** `crates/modules/shipping/Cargo.toml`

```toml
[package]
name = "program1-module-shipping"
version.workspace = true
edition.workspace = true

[dependencies]
program1-contracts = { path = "../../contracts" }
async-trait.workspace = true
serde.workspace = true
serde_json.workspace = true
tracing.workspace = true
reqwest = { version = "0.12", features = ["json"] }
```

### Langkah 4: Tambah DTO & Contract

**File:** `crates/contracts/src/lib.rs`

```rust
// ===== Shipping DTOs =====

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ShippingCourier {
    pub code: String,        // "jne", "tiki", "pos"
    pub name: String,        // "Jalur Nugraha Ekakurir (JNE)"
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ShippingCost {
    pub courier_code: String,
    pub courier_name: String,
    pub service: String,           // "REG", "YES", "OKE"
    pub service_description: String,
    pub cost_cents: i64,
    pub etd: String,               // "2-3" (hari)
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct ShippingCostRequest {
    #[validate(length(min = 1))]
    pub destination_city_id: String,
    #[validate(range(min = 1))]
    pub weight_grams: i32,
    #[validate(length(min = 1))]
    pub courier: String,           // "jne", "tiki", "pos"
}

#[async_trait]
pub trait ShippingContract: Send + Sync {
    /// Get list of supported couriers
    async fn list_couriers(&self) -> Result<Vec<ShippingCourier>, ContractError>;
    /// Calculate shipping cost
    async fn calculate_cost(&self, req: ShippingCostRequest) -> Result<Vec<ShippingCost>, ContractError>;
    /// Search city by name (for autocomplete)
    async fn search_cities(&self, query: &str) -> Result<Vec<ShippingCity>, ContractError>;
}
```

### Langkah 5: Implementasi RajaOngkir Client

**File:** `crates/modules/shipping/src/lib.rs`

```rust
pub struct ShippingModule {
    api_key: String,
    base_url: String,        // https://api.rajaongkir.com/starter atau /basic
    origin_city_id: String,
    client: reqwest::Client,
}

impl ShippingModule {
    pub fn new(api_key: String, api_type: String, origin_city_id: String) -> Self {
        let base_url = match api_type.as_str() {
            "basic" => "https://api.rajaongkir.com/basic",
            "pro" => "https://pro.rajaongkir.com/api",
            _ => "https://api.rajaongkir.com/starter",
        };
        Self {
            api_key,
            base_url: base_url.to_string(),
            origin_city_id,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl ShippingContract for ShippingModule {
    async fn calculate_cost(&self, req: ShippingCostRequest) -> Result<Vec<ShippingCost>, ContractError> {
        let response = self.client
            .post(format!("{}/cost", self.base_url))
            .header("key", &self.api_key)
            .form(&[
                ("origin", self.origin_city_id.as_str()),
                ("destination", &req.destination_city_id),
                ("weight", &req.weight_grams.to_string()),
                ("courier", &req.courier),
            ])
            .send()
            .await
            .map_err(|e| ContractError::ExternalServiceError(e.to_string()))?;

        // Parse RajaOngkir JSON response
        // Map to Vec<ShippingCost>
        // ...
        
        Ok(costs)
    }
}
```

### Langkah 6: Register di Workspace & AppState

**File:** `Cargo.toml` (root) — tambahkan `"crates/modules/shipping"` di `members`

**File:** `crates/web/src/state.rs` — tambahkan `pub shipping_contract: Arc<dyn ShippingContract>`

**File:** `crates/web/src/main.rs` — init `ShippingModule`

### Langkah 7: Tambah Handler & Routes

**File:** `crates/web/src/handlers/shipping.rs` (file baru)

```rust
/// POST /api/v1/shipping/cost — Calculate shipping cost
pub async fn calculate_shipping_handler(
    State(state): State<AppState>,
    Json(req): Json<ShippingCostRequest>,
) -> Result<Json<Vec<ShippingCost>>, ApiError> { /* ... */ }

/// GET /api/v1/shipping/cities?q=jakarta — Search cities
pub async fn search_cities_handler(/* ... */) { /* ... */ }
```

**File:** `crates/web/src/routes.rs`

```rust
// Di buyer_routes:
.route("/api/v1/shipping/cost", post(calculate_shipping_handler))
.route("/api/v1/shipping/cities", get(search_cities_handler))
```

### Langkah 8: Update Checkout Flow

**File:** `crates/web/static/store/store-checkout.js`

Tambahkan step pemilihan kurir di checkout flow:

```javascript
async function loadShippingOptions() {
    const address = getSelectedAddress();
    const totalWeight = calculateCartWeight();
    
    // Show loading
    const couriers = ['jne', 'tiki', 'pos'];
    const allCosts = [];
    
    for (const courier of couriers) {
        const res = await fetch('/api/v1/shipping/cost', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${token}` },
            body: JSON.stringify({
                destination_city_id: address.city_id,
                weight_grams: totalWeight,
                courier: courier,
            })
        });
        const costs = await res.json();
        allCosts.push(...costs);
    }
    
    renderShippingOptions(allCosts);
}
```

### Langkah 9: Update Order Creation

Modifikasi `create_storefront_order` handler untuk menyertakan data shipping:

- Tambahkan `shipping_cost_cents`, `courier_code`, `courier_service` ke `CreateOrderRequest`
- Total order = subtotal + shipping_cost
- Simpan info courier ke order record

### Langkah 10: Admin — Input Resi

Tambahkan di admin order detail: form input tracking number.

**File:** `crates/web/static/admin/admin-orders.js`

```javascript
async function inputTrackingNumber(orderId) {
    const resi = prompt('Masukkan nomor resi pengiriman:');
    if (!resi) return;
    
    const res = await fetch(`/api/v1/orders/${orderId}/tracking`, {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${token}` },
        body: JSON.stringify({ tracking_number: resi })
    });
    // ...
}
```

### Langkah 11: Tulis Unit Test

**File:** `crates/web/tests/shipping_test.rs`

```rust
#[tokio::test]
async fn test_mock_shipping_calculation() {
    // Use MockShippingModule (don't call real API in tests)
    // Verify cost calculation logic
}

#[tokio::test]
async fn test_shipping_cost_added_to_order() {
    // Create order with shipping_cost_cents = 15000
    // Verify total = subtotal + shipping_cost
}

#[tokio::test]
async fn test_tracking_number_update() {
    // Create order, input tracking number
    // Get order → verify tracking_number is set
}
```

### Langkah 12: Verifikasi

```bash
cargo test --workspace
cargo check --workspace
```

---

## ⚠️ Perhatian

- **JANGAN** hardcode API key di source code — WAJIB pakai `.env`
- Di unit test, gunakan `MockShippingModule` — JANGAN panggil RajaOngkir API sungguhan
- RajaOngkir free tier (starter) hanya support JNE, POS, TIKI — sesuaikan
- Berat produk default 500 gram jika belum diset
- Update `.env.example` dengan config shipping baru
- Tambahkan `reqwest` dependency di shipping module `Cargo.toml`
