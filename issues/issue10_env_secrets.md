# Issue #10 — Environment Config & Secrets Management

> **Prioritas**: 🟡 HIGH
> **Estimasi**: 1 hari
> **Depends On**: —
> **Blocks**: Issue #1 (Auth)

---

## 🔍 Masalah Saat Ini

### 1. `.env.example` Terlalu Minim
Saat ini hanya ada:
```env
APP_ENV=development
APP_PORT=6090
STORE_NAME="AURA Storefront"
STORE_CURRENCY="IDR"
RUST_LOG=info,program1=debug,tower_http=debug
```

Tidak ada placeholder untuk database, JWT, atau config keamanan lainnya.

### 2. Port Mismatch
- `.env.example` menyebut `APP_PORT=6090`
- Tapi di `main.rs` default port adalah `8080`
- `docker-compose.yml` menyebut `APP_PORT=8080` (internal) tapi expose `6090:8080`
- Ini bisa membingungkan developer baru

### 3. Environment Variables Tidak Terdokumentasi
Tidak ada penjelasan mana yang wajib, mana yang optional, dan apa default value-nya.

---

## ✅ Acceptance Criteria

### 1. Update `.env.example` Lengkap

```env
# ============================================
# Program1 Environment Configuration
# ============================================
# Copy file ini ke .env dan sesuaikan value-nya.
# JANGAN commit .env ke git!

# --- Application ---
APP_ENV=development          # development | staging | production
APP_PORT=8080                # Internal port (Docker maps 6090:8080)
RUST_LOG=info,program1=debug,tower_http=debug

# --- Store ---
STORE_NAME="AURA Storefront"
STORE_CURRENCY="IDR"

# --- Database (Issue #3) ---
DATABASE_URL=sqlite://./data/program1.db
# Untuk PostgreSQL production:
# DATABASE_URL=postgres://program1_user:secretpassword@localhost:5432/program1
# DB_USER=program1_user
# DB_PASSWORD=changeme_in_production

# --- Authentication (Issue #1 & #2) ---
ADMIN_DEFAULT_PASSWORD=admin123
JWT_SECRET=your-secret-key-minimum-32-characters-change-in-production
JWT_EXPIRY_HOURS=24

# --- Security (Issue #5 & #6) ---
ALLOWED_ORIGINS=http://localhost:8080,http://localhost:3000
# Untuk production:
# ALLOWED_ORIGINS=https://yourdomain.com

# --- Rate Limiting (Issue #6) ---
RATE_LIMIT_PER_SECOND=100
LOGIN_RATE_LIMIT_PER_MINUTE=5
```

### 2. Buat Config Struct di `crates/core/src/config.rs`

```rust
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub app_env: String,
    pub app_port: u16,
    pub store_name: String,
    pub store_currency: String,
    pub database_url: Option<String>,
    pub admin_default_password: String,
    pub jwt_secret: String,
    pub jwt_expiry_hours: u64,
    pub allowed_origins: Vec<String>,
    pub rate_limit_per_second: u64,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            app_env: env_or("APP_ENV", "development"),
            app_port: env_or("APP_PORT", "8080").parse().unwrap_or(8080),
            store_name: env_or("STORE_NAME", "AURA Storefront"),
            store_currency: env_or("STORE_CURRENCY", "IDR"),
            database_url: std::env::var("DATABASE_URL").ok(),
            admin_default_password: env_or("ADMIN_DEFAULT_PASSWORD", "admin123"),
            jwt_secret: env_or("JWT_SECRET", "CHANGE_ME_IN_PRODUCTION_minimum_32_chars!!"),
            jwt_expiry_hours: env_or("JWT_EXPIRY_HOURS", "24").parse().unwrap_or(24),
            allowed_origins: env_or("ALLOWED_ORIGINS", "http://localhost:8080")
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            rate_limit_per_second: env_or("RATE_LIMIT_PER_SECOND", "100").parse().unwrap_or(100),
        }
    }

    pub fn is_production(&self) -> bool {
        self.app_env == "production"
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
```

### 3. Startup Validation

Di `main.rs`, saat startup:

```rust
let config = AppConfig::from_env();

// Warn jika JWT secret masih default di production
if config.is_production() && config.jwt_secret.contains("CHANGE_ME") {
    tracing::error!("⚠️  JWT_SECRET masih menggunakan default value! HARUS diubah di production!");
    std::process::exit(1);
}

// Warn jika admin password masih default
if config.is_production() && config.admin_default_password == "admin123" {
    tracing::warn!("⚠️  ADMIN_DEFAULT_PASSWORD masih 'admin123'. Segera ubah setelah login pertama!");
}

tracing::info!(
    env = %config.app_env,
    port = %config.app_port,
    store = %config.store_name,
    "Program1 configuration loaded"
);
```

### 4. Update `check_dependencies.sh`

Tambahkan check untuk critical env vars:

```bash
# 5. Check critical environment variables
if [ -f ".env" ]; then
    if grep -q "CHANGE_ME" .env; then
        echo "[WARN] .env contains default JWT_SECRET. Change before production!"
    fi
    if grep -q "admin123" .env; then
        echo "[WARN] .env contains default admin password. Change before production!"
    fi
fi
```

### 5. Update `.gitignore`

Pastikan:
```gitignore
.env
*.db
data/
```

---

## 🧪 Unit Test

```rust
#[cfg(test)]
mod tests {
    // 1. Test config dari default env
    #[test]
    fn test_default_config() { ... }
    
    // 2. Test is_production check
    #[test]
    fn test_is_production() { ... }
    
    // 3. Test CORS origins parsing
    #[test]
    fn test_cors_origins_parsing() { ... }
}
```

---

## ⚠️ Peringatan

- JANGAN commit `.env` ke git — hanya `.env.example`
- Docker Compose sudah pakai `env_file: - .env` — ini benar
- Semua existing test `cargo test --workspace` harus tetap PASS
