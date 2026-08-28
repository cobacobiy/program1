# Issue #2 — JWT Middleware & Route Protection

> **Prioritas**: 🔴 CRITICAL
> **Estimasi**: 2-3 hari
> **Depends On**: Issue #1 (Authentication & Password Hashing)
> **Blocks**: —

---

## 🔍 Masalah Saat Ini

Semua API endpoint di `crates/web/src/main.rs` **terbuka tanpa proteksi**:
- `POST /api/v1/catalog` — siapapun bisa tambah produk
- `POST /api/v1/users/accounts` — siapapun bisa buat user
- `POST /api/v1/inventory/:id/safety-stock` — siapapun bisa ubah safety stock
- `GET /api/v1/analytics` — data bisnis terbuka untuk publik

Tidak ada middleware yang memverifikasi token atau role user.

### File Terkait:
- `crates/web/src/main.rs` — router & handlers (baris 79-103)
- `crates/contracts/src/lib.rs` — belum ada AuthContract

---

## ✅ Acceptance Criteria

### 1. Tambahkan Dependencies

Di `[workspace.dependencies]` pada `Cargo.toml` root:

```toml
jsonwebtoken = "9.3"   # JWT encode/decode
```

### 2. Buat Auth Module Baru: `crates/modules/auth/`

Struktur:
```
crates/modules/auth/
├── Cargo.toml
└── src/
    └── lib.rs
```

`Cargo.toml`:
```toml
[package]
name = "program1-module-auth"
version.workspace = true
edition.workspace = true

[dependencies]
program1-contracts = { path = "../../contracts" }
jsonwebtoken = { workspace = true }
serde.workspace = true
chrono.workspace = true
uuid.workspace = true
```

### 3. Definisikan `AuthContract` di `crates/contracts/src/lib.rs`

```rust
/// JWT Claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: Uuid,           // user_id
    pub username: String,
    pub role: String,
    pub accessible_menus: Vec<String>,
    pub exp: i64,            // expiry timestamp
    pub iat: i64,            // issued at
}

#[async_trait]
pub trait AuthContract: Send + Sync {
    /// Generate JWT token dari UserAccountDto
    fn generate_token(&self, user: &UserAccountDto) -> Result<String, ContractError>;
    
    /// Validate & decode JWT token
    fn validate_token(&self, token: &str) -> Result<JwtClaims, ContractError>;
}
```

### 4. Implementasi AuthModule

Di `crates/modules/auth/src/lib.rs`:

```rust
pub struct AuthModule {
    jwt_secret: String,
    token_expiry_hours: u64,
}

impl AuthModule {
    pub fn new(jwt_secret: String, token_expiry_hours: u64) -> Self {
        Self { jwt_secret, token_expiry_hours }
    }
}
```

- `jwt_secret` diambil dari env var `JWT_SECRET`
- `token_expiry_hours` default 24 jam, configurable via `JWT_EXPIRY_HOURS`

### 5. Buat Axum Middleware Extractor

Di `crates/web/src/main.rs` (atau file terpisah `crates/web/src/middleware.rs`):

```rust
/// Extractor untuk authenticated user
pub struct AuthUser(pub JwtClaims);

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // 1. Extract "Authorization: Bearer <token>" header
        // 2. Validate token via auth_contract.validate_token()
        // 3. Return AuthUser(claims) or 401
    }
}

/// Extractor untuk role-based access
pub struct RequireRole<const ROLE: &'static str>(pub JwtClaims);
```

### 6. Klasifikasi Route Protection

| Endpoint | Protection Level | Keterangan |
|----------|-----------------|------------|
| `GET /health` | 🟢 Public | Health check |
| `GET /` dan `/store` | 🟢 Public | Storefront page |
| `POST /api/v1/auth/login` | 🟢 Public | Login |
| `POST /api/v1/auth/register` | 🟡 Admin Only | Hanya admin bisa buat user |
| `GET /api/v1/catalog` | 🟢 Public | Lihat produk (storefront) |
| `POST /api/v1/catalog` | 🔴 Auth Required | Tambah produk |
| `GET /api/v1/inventory` | 🔴 Auth Required | Lihat stock |
| `POST /api/v1/inventory/:id/safety-stock` | 🔴 Auth + Role | Warehouse Manager / Admin |
| `GET /api/v1/users/accounts` | 🔴 Auth + Admin | Lihat semua user |
| `POST /api/v1/orders` | 🟢 Public | Storefront checkout |
| `GET /api/v1/orders` | 🔴 Auth Required | Lihat semua order |
| `GET /api/v1/analytics` | 🔴 Auth + Admin | Data bisnis |
| `GET /admin` | 🔴 Auth Required | Admin dashboard |

### 7. Update Router di `main.rs`

Pisahkan routes menjadi groups:

```rust
// Public routes (no auth)
let public_routes = Router::new()
    .route("/health", get(health_check))
    .route("/api/v1/auth/login", post(login_handler))
    .route("/api/v1/catalog", get(list_catalog))
    .route("/api/v1/orders", post(create_storefront_order));

// Protected routes (auth required)
let protected_routes = Router::new()
    .route("/api/v1/catalog", post(create_catalog_item))
    .route("/api/v1/inventory", get(list_all_inventory))
    .route("/api/v1/orders", get(list_orders))
    // ... dll
    .layer(middleware::from_fn(require_auth));

// Admin-only routes
let admin_routes = Router::new()
    .route("/api/v1/auth/register", post(register_handler))
    .route("/api/v1/users/accounts", get(list_user_accounts))
    .route("/api/v1/analytics", get(get_analytics))
    .layer(middleware::from_fn(require_admin));
```

### 8. Error Responses

```json
// 401 Unauthorized
{
    "error": "authentication_required",
    "message": "Missing or invalid Authorization header"
}

// 403 Forbidden
{
    "error": "insufficient_permissions",
    "message": "Your role does not have access to this resource"
}

// 401 Token Expired
{
    "error": "token_expired",
    "message": "Your session has expired. Please login again"
}
```

### 9. Update Workspace

Tambahkan di root `Cargo.toml`:
```toml
members = [
    # ... existing
    "crates/modules/auth",
]
```

Dan di `crates/web/Cargo.toml`:
```toml
program1-module-auth = { path = "../modules/auth" }
axum = { workspace = true, features = ["macros"] }
axum-extra = { version = "0.9", features = ["typed-header"] }
```

### 10. Update `.env.example`

```env
JWT_SECRET=your-secret-key-minimum-32-characters-change-in-production
JWT_EXPIRY_HOURS=24
```

---

## 🧪 Unit Test yang Harus Dibuat

```rust
#[cfg(test)]
mod tests {
    // 1. Test generate & validate token
    #[test]
    fn test_jwt_roundtrip() { ... }
    
    // 2. Test token expired
    #[test]
    fn test_expired_token_rejected() { ... }
    
    // 3. Test invalid token format
    #[test]
    fn test_invalid_token_rejected() { ... }
    
    // 4. Test tampered token (wrong secret)
    #[test]
    fn test_tampered_token_rejected() { ... }
    
    // 5. Test claims extraction
    #[test]
    fn test_claims_contain_user_info() { ... }
}
```

---

## ⚠️ Peringatan

- `JWT_SECRET` HARUS minimal 32 karakter di production
- JANGAN hardcode secret di source code
- Token harus di-include di header `Authorization: Bearer <token>`
- Storefront checkout (`POST /api/v1/orders`) tetap public karena customer belum tentu punya akun
- Semua existing test `cargo test --workspace` harus tetap PASS

---

## 📎 Referensi

- [jsonwebtoken crate docs](https://docs.rs/jsonwebtoken/latest/jsonwebtoken/)
- [Axum middleware guide](https://docs.rs/axum/latest/axum/middleware/index.html)
- [OWASP JWT Security Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/JSON_Web_Token_for_Java_Cheat_Sheet.html)
