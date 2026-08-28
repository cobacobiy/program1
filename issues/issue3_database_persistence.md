# Issue #3 — Database Persistence (SQLite → PostgreSQL)

> **Prioritas**: 🔴 CRITICAL
> **Estimasi**: 4-5 hari
> **Depends On**: Issue #1 (Auth), Issue #4 (Input Validation)
> **Blocks**: Issue #7 (Audit Logging)

---

## 🔍 Masalah Saat Ini

**Semua data disimpan di in-memory `HashMap`** yang dibungkus `Arc<RwLock<...>>`. Artinya:
- ❌ Semua data hilang saat aplikasi restart atau container di-recreate
- ❌ Tidak bisa scale horizontal (data terisolasi per instance)
- ❌ Tidak ada data persistence untuk orders, inventory adjustments, audit logs

### Modul yang Terpengaruh:
| Module | Storage | File |
|--------|---------|------|
| `user` | `HashMap<Uuid, UserAccountDto>` | `crates/modules/user/src/lib.rs` |
| `catalog` | `HashMap<Uuid, CatalogItemDto>` | `crates/modules/catalog/src/lib.rs` |
| `inventory` | `HashMap<Uuid, InventoryStockDto>` + `HashMap<Uuid, Vec<SafetyStockLogDto>>` | `crates/modules/inventory/src/lib.rs` |
| `channel` | `HashMap<ChannelType, ChannelStatusDto>` | `crates/modules/channel/src/lib.rs` |
| `order` | `HashMap<Uuid, OmniOrderDto>` | `crates/modules/order/src/lib.rs` |

---

## ✅ Acceptance Criteria

### Strategi: SQLite untuk Development, PostgreSQL untuk Production

Gunakan **SQLx** sebagai async database driver karena:
- Compile-time query checking
- Mendukung SQLite + PostgreSQL dengan interface yang sama
- Async native (Tokio compatible)

### 1. Tambahkan Dependencies

Di `[workspace.dependencies]` pada `Cargo.toml` root:

```toml
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "postgres", "uuid", "chrono", "migrate"] }
```

### 2. Buat Database Module: `crates/core/src/database.rs`

```rust
use sqlx::{Pool, Sqlite, Postgres};

pub enum DatabasePool {
    Sqlite(Pool<Sqlite>),
    Postgres(Pool<Postgres>),
}

/// Initialize database pool berdasarkan DATABASE_URL
pub async fn init_database(database_url: &str) -> Result<DatabasePool, sqlx::Error> {
    if database_url.starts_with("sqlite") {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        sqlx::migrate!("./migrations/sqlite").run(&pool).await?;
        Ok(DatabasePool::Sqlite(pool))
    } else {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        sqlx::migrate!("./migrations/postgres").run(&pool).await?;
        Ok(DatabasePool::Postgres(pool))
    }
}
```

### 3. Migration Files

Buat folder `migrations/` di root project:

```
migrations/
├── sqlite/
│   ├── 001_create_users.sql
│   ├── 002_create_catalog.sql
│   ├── 003_create_inventory.sql
│   ├── 004_create_orders.sql
│   └── 005_create_channels.sql
└── postgres/
    ├── 001_create_users.sql
    ├── ... (same structure)
```

#### Contoh `001_create_users.sql`:

```sql
CREATE TABLE IF NOT EXISTS user_accounts (
    id TEXT PRIMARY KEY,          -- UUID as text (SQLite) / UUID type (Postgres)
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    full_name TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'Staff',
    accessible_menus TEXT NOT NULL DEFAULT '[]',  -- JSON array
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Seed admin user (password hash for "admin123")
-- Hash harus di-generate saat runtime, jadi seed dilakukan di code
```

#### Contoh `002_create_catalog.sql`:

```sql
CREATE TABLE IF NOT EXISTS catalog_items (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    sku TEXT NOT NULL UNIQUE,
    category TEXT NOT NULL DEFAULT 'General',
    price REAL NOT NULL CHECK(price >= 0),
    stock INTEGER NOT NULL DEFAULT 0 CHECK(stock >= 0),
    image_url TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

#### Contoh `003_create_inventory.sql`:

```sql
CREATE TABLE IF NOT EXISTS inventory_stocks (
    product_id TEXT PRIMARY KEY REFERENCES catalog_items(id),
    sku TEXT NOT NULL,
    product_name TEXT NOT NULL,
    image_url TEXT NOT NULL DEFAULT '',
    average_purchase_price REAL NOT NULL DEFAULT 0,
    warehouse_stock INTEGER NOT NULL DEFAULT 0,
    spare_stock INTEGER NOT NULL DEFAULT 0,
    locked_stock INTEGER NOT NULL DEFAULT 0,
    promotion_stock INTEGER NOT NULL DEFAULT 0,
    safety_stock INTEGER NOT NULL DEFAULT 0,
    available_stock INTEGER NOT NULL DEFAULT 0,
    last_updated TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS safety_stock_logs (
    id TEXT PRIMARY KEY,
    product_id TEXT NOT NULL REFERENCES catalog_items(id),
    old_safety_stock INTEGER NOT NULL,
    new_safety_stock INTEGER NOT NULL,
    admin_note TEXT NOT NULL,
    updated_by TEXT NOT NULL DEFAULT 'System',
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

#### Contoh `004_create_orders.sql`:

```sql
CREATE TABLE IF NOT EXISTS orders (
    id TEXT PRIMARY KEY,
    channel TEXT NOT NULL DEFAULT 'NativeWeb',
    customer_name TEXT NOT NULL,
    customer_email TEXT NOT NULL DEFAULT '',
    shipping_address TEXT NOT NULL DEFAULT '',
    total_amount REAL NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'PENDING',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS order_items (
    id TEXT PRIMARY KEY,
    order_id TEXT NOT NULL REFERENCES orders(id),
    product_id TEXT NOT NULL,
    product_name TEXT NOT NULL,
    quantity INTEGER NOT NULL CHECK(quantity > 0),
    unit_price REAL NOT NULL,
    total_price REAL NOT NULL
);
```

### 4. Update Setiap Module

Untuk setiap module, ubah constructor agar menerima `DatabasePool`:

```rust
// SEBELUM:
pub struct CatalogModule {
    store: Arc<RwLock<HashMap<Uuid, CatalogItemDto>>>,
}

impl CatalogModule {
    pub fn new() -> Self { ... }
}

// SESUDAH:
pub struct CatalogModule {
    pool: sqlx::Pool<sqlx::Sqlite>, // atau generic
}

impl CatalogModule {
    pub fn new(pool: sqlx::Pool<sqlx::Sqlite>) -> Self {
        Self { pool }
    }
}
```

### 5. Pendekatan Migrasi yang Aman

**Tahap 1**: Tambahkan database di belakang feature flag
```toml
[features]
default = ["in-memory"]
in-memory = []
database = ["sqlx"]
```

**Tahap 2**: Buat implementasi database terpisah (jangan hapus in-memory dulu)

**Tahap 3**: Switch default ke database setelah semua test pass

### 6. Update Docker Compose

```yaml
services:
  program1-app:
    # ... existing config
    environment:
      - DATABASE_URL=sqlite:///app/data/program1.db
    volumes:
      - program1_data:/app/data

volumes:
  program1_data:
    name: program1_data
```

Untuk PostgreSQL production:
```yaml
services:
  program1-db:
    image: postgres:16-alpine
    container_name: program1-db
    restart: unless-stopped
    environment:
      POSTGRES_DB: program1
      POSTGRES_USER: ${DB_USER}
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes:
      - pgdata:/var/lib/postgresql/data
    ports:
      - "5432:5432"

  program1-app:
    depends_on:
      - program1-db
    environment:
      - DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@program1-db:5432/program1
```

### 7. Update `.env.example`

```env
# Database
DATABASE_URL=sqlite://./data/program1.db
# Untuk PostgreSQL production:
# DATABASE_URL=postgres://program1_user:secretpassword@localhost:5432/program1
DB_USER=program1_user
DB_PASSWORD=changeme_in_production
```

### 8. Seed Data

Data seed (initial products, admin user) yang saat ini di-hardcode di `new()` harus dipindahkan ke:
- Migration SQL script (untuk data yang fixed)
- Runtime seeding function yang cek apakah data sudah ada

---

## 🧪 Unit Test yang Harus Dibuat

```rust
#[cfg(test)]
mod tests {
    // Gunakan SQLite in-memory untuk testing
    // DATABASE_URL = "sqlite::memory:"
    
    // 1. Test database initialization & migration
    #[tokio::test]
    async fn test_database_init() { ... }
    
    // 2. Test CRUD catalog via database
    #[tokio::test]
    async fn test_catalog_db_crud() { ... }
    
    // 3. Test inventory stock persistence
    #[tokio::test]
    async fn test_inventory_db_persistence() { ... }
    
    // 4. Test order creation & retrieval
    #[tokio::test]
    async fn test_order_db_roundtrip() { ... }
    
    // 5. Test concurrent access
    #[tokio::test]
    async fn test_concurrent_stock_reservation() { ... }
}
```

---

## ⚠️ Peringatan

- Gunakan `sqlx::migrate!()` macro untuk embedded migrations
- JANGAN pakai `sqlx::query!()` macro di awal karena butuh live database saat compile — gunakan `sqlx::query_as()` atau `sqlx::query()` dulu
- SQLite file harus di-mount sebagai Docker volume agar persist
- Transactions WAJIB digunakan untuk operasi yang melibatkan multiple tables (contoh: create order + reserve stock)
- Jangan lupa `PRAGMA journal_mode=WAL;` untuk SQLite concurrent reads
- Semua existing test `cargo test --workspace` harus tetap PASS (feature flag in-memory)

---

## 📎 Referensi

- [SQLx docs](https://docs.rs/sqlx/latest/sqlx/)
- [SQLx migrations guide](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md)
- [SQLite WAL mode](https://www.sqlite.org/wal.html)
