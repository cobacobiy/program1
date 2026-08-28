# Issue #4 — Input Validation & Sanitization

> **Prioritas**: 🟡 HIGH
> **Estimasi**: 2 hari
> **Depends On**: —
> **Blocks**: Issue #3 (Database — harus sanitize sebelum persist)

---

## 🔍 Masalah Saat Ini

Validasi input saat ini **sangat minimal**:
- Hanya cek `is_empty()` dan `price < 0`
- Tidak ada length limit — user bisa submit string 10MB
- Tidak ada sanitization terhadap HTML/script injection
- Tidak ada validasi format email
- Tidak ada validasi UUID format sebelum query
- Quantity bisa 0 (sudah di-cek) tapi tidak ada upper limit

### Contoh Risiko:
```json
// Ini akan diterima:
{
    "customer_name": "<script>alert('xss')</script>",
    "customer_email": "bukan-email",
    "shipping_address": "",
    "items": [{"product_id": "bukan-uuid", "quantity": 999999}]
}
```

### File Terkait:
- `crates/contracts/src/lib.rs` — DTO definitions
- `crates/modules/*/src/lib.rs` — semua module handlers
- `crates/web/src/main.rs` — request handlers

---

## ✅ Acceptance Criteria

### 1. Tambahkan Dependencies

Di `[workspace.dependencies]`:

```toml
validator = { version = "0.18", features = ["derive"] }
```

### 2. Tambahkan Validasi di DTO (`crates/contracts/src/lib.rs`)

Gunakan `validator` derive macro:

```rust
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateCatalogItemRequest {
    #[validate(length(min = 1, max = 200, message = "Name must be 1-200 characters"))]
    pub name: String,
    
    #[validate(length(min = 1, max = 50, message = "SKU must be 1-50 characters"))]
    pub sku: String,
    
    #[validate(length(max = 100))]
    pub category: String,
    
    #[validate(range(min = 0.0, max = 999999999.0, message = "Price must be 0 - 999,999,999"))]
    pub price: f64,
    
    #[validate(range(max = 999999, message = "Stock cannot exceed 999,999"))]
    pub stock: u32,
    
    #[validate(url(message = "Invalid image URL format"))]
    pub image_url: Option<String>,
    
    #[validate(length(max = 2000))]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct StorefrontOrderRequest {
    #[validate(length(min = 1, max = 200, message = "Customer name required (max 200 chars)"))]
    pub customer_name: String,
    
    #[validate(email(message = "Invalid email format"))]
    pub customer_email: String,
    
    #[validate(length(min = 1, max = 500, message = "Shipping address required (max 500 chars)"))]
    pub shipping_address: String,
    
    #[validate(length(min = 1, max = 50, message = "Order must have 1-50 items"))]
    pub items: Vec<StorefrontOrderItemRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct StorefrontOrderItemRequest {
    pub product_id: Uuid,
    
    #[validate(range(min = 1, max = 9999, message = "Quantity must be 1-9999"))]
    pub quantity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateUserAccountRequest {
    #[validate(length(min = 3, max = 50, message = "Username must be 3-50 characters"))]
    #[validate(regex(path = "USERNAME_REGEX", message = "Username must be alphanumeric/underscore only"))]
    pub username: String,
    
    #[validate(length(min = 1, max = 200))]
    pub full_name: String,
    
    #[validate(length(min = 1, max = 50))]
    pub role: String,
    
    pub accessible_menus: Vec<String>,
}
```

Tambahkan regex constant:
```rust
use once_cell::sync::Lazy;
use regex::Regex;

static USERNAME_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9_]+$").unwrap()
});
```

### 3. Buat Validation Helper di Handler

Di `crates/web/src/main.rs`, buat wrapper:

```rust
use validator::Validate;

/// Validated JSON extractor — automatically runs validation
pub struct ValidatedJson<T>(pub T);

#[axum::async_trait]
impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))))?;
        
        value.validate()
            .map_err(|e| {
                let errors: Vec<String> = e.field_errors()
                    .iter()
                    .flat_map(|(field, errors)| {
                        errors.iter().map(move |err| {
                            format!("{}: {}", field, err.message.as_ref().unwrap_or(&std::borrow::Cow::Borrowed("invalid")))
                        })
                    })
                    .collect();
                
                (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({
                    "error": "validation_failed",
                    "details": errors
                })))
            })?;
        
        Ok(ValidatedJson(value))
    }
}
```

### 4. Update Handlers untuk Menggunakan `ValidatedJson`

```rust
// SEBELUM:
async fn create_catalog_item(
    State(state): State<AppState>,
    Json(payload): Json<CreateCatalogItemRequest>,
) -> impl IntoResponse { ... }

// SESUDAH:
async fn create_catalog_item(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateCatalogItemRequest>,
) -> impl IntoResponse { ... }
```

### 5. Sanitization Functions

Buat di `crates/core/src/sanitize.rs`:

```rust
/// Strip HTML tags dari string
pub fn strip_html(input: &str) -> String {
    // Simple regex-based HTML stripping
    let re = Regex::new(r"<[^>]*>").unwrap();
    re.replace_all(input, "").to_string()
}

/// Trim & sanitize text input
pub fn sanitize_text(input: &str, max_length: usize) -> String {
    let trimmed = input.trim();
    let stripped = strip_html(trimmed);
    if stripped.len() > max_length {
        stripped[..max_length].to_string()
    } else {
        stripped
    }
}
```

### 6. Request Body Size Limit

Di `crates/web/src/main.rs`, tambahkan body size limit:

```rust
use axum::extract::DefaultBodyLimit;
use tower_http::limit::RequestBodyLimitLayer;

let app = Router::new()
    // ... routes
    .layer(DefaultBodyLimit::max(1024 * 1024)) // 1MB max body
    .layer(cors);
```

---

## 🧪 Unit Test yang Harus Dibuat

```rust
#[cfg(test)]
mod tests {
    // 1. Test name terlalu panjang ditolak
    #[test]
    fn test_name_too_long_rejected() { ... }
    
    // 2. Test email format invalid ditolak
    #[test]
    fn test_invalid_email_rejected() { ... }
    
    // 3. Test quantity 0 ditolak
    #[test]
    fn test_zero_quantity_rejected() { ... }
    
    // 4. Test price negatif ditolak
    #[test]
    fn test_negative_price_rejected() { ... }
    
    // 5. Test HTML stripping
    #[test]
    fn test_html_stripped() { ... }
    
    // 6. Test username regex
    #[test]
    fn test_username_special_chars_rejected() { ... }
    
    // 7. Test empty items array ditolak
    #[test]
    fn test_empty_order_items_rejected() { ... }
}
```

---

## ⚠️ Peringatan

- Validasi harus terjadi di **DUA tempat**: handler (DTO validation) DAN module (business logic validation)
- Jangan hapus existing validasi di module code — tambahkan di atasnya
- `validator` crate validation errors harus di-format dengan baik untuk frontend
- Semua existing test `cargo test --workspace` harus tetap PASS

---

## 📎 Referensi

- [validator crate docs](https://docs.rs/validator/latest/validator/)
- [OWASP Input Validation Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Input_Validation_Cheat_Sheet.html)
