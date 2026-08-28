# Issue #7 — Audit Logging & Activity Trail

> **Prioritas**: 🟢 MEDIUM
> **Estimasi**: 2 hari
> **Depends On**: Issue #3 (Database Persistence)
> **Blocks**: —

---

## 🔍 Masalah Saat Ini

Saat ini hanya ada `tracing::info!()` basic logging ke stdout:
- Tidak ada catatan siapa melakukan apa dan kapan
- Safety stock log sudah ada tapi hanya di memory (hilang saat restart)
- Tidak ada trail untuk debugging dan compliance
- Tidak tahu siapa yang membuat/mengubah data

---

## ✅ Acceptance Criteria

### 1. Definisikan `AuditContract` di contracts

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub actor_id: Option<Uuid>,      // User yang melakukan aksi (None = system/anonymous)
    pub actor_username: String,       // "admin", "system", "anonymous"
    pub action: String,               // "CREATE_ORDER", "UPDATE_SAFETY_STOCK", "LOGIN", etc.
    pub resource_type: String,        // "order", "catalog_item", "inventory", "user"
    pub resource_id: Option<Uuid>,    // ID resource yang terpengaruh
    pub details: String,              // JSON string dengan detail tambahan
    pub ip_address: Option<String>,   // IP address requester
}

#[async_trait]
pub trait AuditContract: Send + Sync {
    async fn log_action(&self, entry: AuditLogEntry) -> Result<(), ContractError>;
    async fn get_logs(
        &self,
        resource_type: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AuditLogEntry>, ContractError>;
    async fn get_logs_by_actor(&self, actor_id: Uuid) -> Result<Vec<AuditLogEntry>, ContractError>;
}
```

### 2. Buat `AuditModule`

`crates/modules/audit/` — implementasi baru yang menyimpan ke database (setelah Issue #3 selesai).

Sementara bisa pakai in-memory dulu dengan append-only Vec.

### 3. Aksi yang HARUS Di-audit

| Action | Resource Type | Keterangan |
|--------|--------------|------------|
| `LOGIN_SUCCESS` | user | Login berhasil |
| `LOGIN_FAILED` | user | Login gagal (catat username) |
| `USER_CREATED` | user | User baru dibuat |
| `PERMISSIONS_UPDATED` | user | Permission user diubah |
| `CATALOG_ITEM_CREATED` | catalog | Produk baru ditambah |
| `SAFETY_STOCK_UPDATED` | inventory | Safety stock diubah |
| `ORDER_CREATED` | order | Order baru dibuat |
| `CHANNEL_SYNCED` | channel | Channel stock disync |

### 4. API Endpoint Audit Logs

```
GET /api/v1/audit/logs?limit=50&offset=0&resource_type=order  → list audit logs
GET /api/v1/audit/logs/user/:user_id                           → logs by user
```

Endpoint ini harus **Admin Only** (protected by JWT middleware dari Issue #2).

### 5. Database Table (setelah Issue #3)

```sql
CREATE TABLE IF NOT EXISTS audit_logs (
    id TEXT PRIMARY KEY,
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    actor_id TEXT,
    actor_username TEXT NOT NULL DEFAULT 'system',
    action TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT,
    details TEXT NOT NULL DEFAULT '{}',
    ip_address TEXT
);

CREATE INDEX idx_audit_timestamp ON audit_logs(timestamp DESC);
CREATE INDEX idx_audit_resource ON audit_logs(resource_type, resource_id);
CREATE INDEX idx_audit_actor ON audit_logs(actor_id);
```

### 6. Update Workspace

Tambahkan `crates/modules/audit` ke `Cargo.toml`:
```toml
members = [
    # ... existing
    "crates/modules/audit",
]
```

---

## 🧪 Unit Test

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_audit_log_creation() { ... }
    
    #[tokio::test]
    async fn test_audit_log_query_by_resource() { ... }
    
    #[tokio::test]
    async fn test_audit_log_query_by_actor() { ... }
}
```

---

## ⚠️ Peringatan

- JANGAN log sensitive data (password, token) di audit log
- Audit log harus append-only — JANGAN bisa di-delete via API
- Gunakan pagination (limit/offset) untuk query — jangan return semua
- Semua existing test `cargo test --workspace` harus tetap PASS
