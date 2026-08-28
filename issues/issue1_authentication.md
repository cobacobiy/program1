# Issue #1 — Authentication & Password Hashing

> **Prioritas**: 🔴 CRITICAL
> **Estimasi**: 3-4 hari
> **Depends On**: Issue #10 (Env & Secrets)
> **Blocks**: Issue #2 (JWT Middleware)

---

## 🔍 Masalah Saat Ini

Saat ini `UserAccountDto` **tidak memiliki field password sama sekali**. Artinya:
- Siapapun bisa mengakses semua API endpoint tanpa login
- Tidak ada mekanisme login/register
- Tidak ada credential verification
- Data user hanya berisi `username`, `full_name`, `role` — tanpa password hash

### File Terkait:
- `crates/contracts/src/lib.rs` — DTO & trait definitions (baris 46-76)
- `crates/modules/user/src/lib.rs` — UserModule implementation
- `crates/web/src/main.rs` — HTTP handlers (baris 175-201)

---

## ✅ Acceptance Criteria

### 1. Tambahkan Dependencies Baru

Tambahkan di `[workspace.dependencies]` pada `Cargo.toml` root:

```toml
argon2 = "0.5"           # Password hashing (Argon2id)
rand = "0.8"              # Random salt generation
```

Dan di `crates/contracts/Cargo.toml`:
```toml
[dependencies]
argon2 = { workspace = true }
rand = { workspace = true }
```

### 2. Update Contract DTOs di `crates/contracts/src/lib.rs`

**JANGAN ubah `UserAccountDto`** — DTO response tidak boleh berisi password. Tambahkan DTO baru:

```rust
/// Request DTO untuk login
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Response DTO setelah login berhasil
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTokenResponse {
    pub access_token: String,
    pub token_type: String, // "Bearer"
    pub expires_in: u64,    // seconds
    pub user: UserAccountDto,
}

/// Request DTO untuk register (extend CreateUserAccountRequest)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterUserRequest {
    pub username: String,
    pub password: String,
    pub full_name: String,
    pub role: String,
    pub accessible_menus: Vec<String>,
}
```

### 3. Tambahkan Method Baru di `UserContract` Trait

```rust
#[async_trait]
pub trait UserContract: Send + Sync {
    // ... existing methods ...
    
    /// Authenticate user — returns account if credentials valid
    async fn authenticate(&self, username: &str, password: &str) -> Result<UserAccountDto, ContractError>;
    
    /// Register user baru dengan password
    async fn register(&self, req: RegisterUserRequest) -> Result<UserAccountDto, ContractError>;
}
```

### 4. Update Internal Model di `crates/modules/user/src/lib.rs`

Buat internal struct terpisah (TIDAK di-expose ke contract):

```rust
/// Internal only — TIDAK di-export
struct UserAccountInternal {
    pub dto: UserAccountDto,
    pub password_hash: String,  // Argon2id hash
}
```

Implementasi:
- `UserModule.accounts` berubah dari `HashMap<Uuid, UserAccountDto>` menjadi `HashMap<Uuid, UserAccountInternal>`
- Tambahkan `HashMap<String, Uuid>` sebagai username index untuk lookup cepat
- Seed admin user dengan password default dari env var `ADMIN_DEFAULT_PASSWORD` (default: `"admin123"`)

### 5. Password Hashing Utility

Buat helper functions (bisa di `crates/core/src/lib.rs` atau file baru `crates/core/src/auth.rs`):

```rust
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| format!("Password hashing failed: {}", e))
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, String> {
    let parsed = PasswordHash::new(hash)
        .map_err(|e| format!("Invalid hash format: {}", e))?;
    Ok(Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
}
```

### 6. Tambahkan HTTP Endpoint di `crates/web/src/main.rs`

```
POST /api/v1/auth/login     → login handler
POST /api/v1/auth/register  → register handler (bisa di-restrict nanti)
```

### 7. Password Validation Rules

- Minimum 8 karakter
- Harus mengandung huruf besar, huruf kecil, dan angka
- TIDAK boleh sama dengan username

### 8. Update `.env.example`

```env
ADMIN_DEFAULT_PASSWORD=admin123
JWT_SECRET=your-secret-key-change-me-in-production
```

---

## 🧪 Unit Test yang Harus Dibuat

```rust
#[cfg(test)]
mod tests {
    // 1. Test password hashing & verification
    #[test]
    fn test_hash_and_verify_password() { ... }
    
    // 2. Test login dengan credential benar
    #[tokio::test]
    async fn test_authenticate_valid_credentials() { ... }
    
    // 3. Test login dengan password salah
    #[tokio::test]
    async fn test_authenticate_wrong_password() { ... }
    
    // 4. Test login dengan username tidak ada
    #[tokio::test]
    async fn test_authenticate_nonexistent_user() { ... }
    
    // 5. Test register user baru
    #[tokio::test]
    async fn test_register_new_user() { ... }
    
    // 6. Test register dengan username duplikat
    #[tokio::test]
    async fn test_register_duplicate_username() { ... }
    
    // 7. Test password validation rules
    #[test]
    fn test_password_too_short() { ... }
}
```

---

## ⚠️ Peringatan

- **JANGAN** simpan password dalam plaintext — selalu hash dengan Argon2id
- **JANGAN** kembalikan password hash di response API
- **JANGAN** log password di tracing/logging
- Seed admin password harus bisa di-override via environment variable
- Semua existing test di `cargo test --workspace` harus tetap PASS

---

## 📎 Referensi

- [Argon2 Rust Crate](https://docs.rs/argon2/latest/argon2/)
- [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
