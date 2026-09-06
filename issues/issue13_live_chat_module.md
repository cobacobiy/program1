# Issue #13 — Live Chat Module (Backend Implementation)

> **Prioritas**: 🟡 HIGH
> **Estimasi**: 3-4 hari
> **Depends On**: —
> **Skill Level**: Junior-Mid Rust Developer

---

## 🔍 Masalah Saat Ini

`ChatContract` sudah didefinisikan lengkap di `crates/contracts/src/lib.rs` (baris 901-967) dengan DTOs:
- `ChatMessageDto`, `ChatRoomDto`, `SendMessageRequest`
- `ChatSenderType` enum: `Buyer`, `Seller`, `System`

**Tapi belum ada module implementasi backend-nya.** Saat ini fitur "In-App Live Chat" di storefront hanya menyimpan pesan di `localStorage` browser — **tidak persisten dan tidak bisa dilihat seller di admin panel**.

### Contract Yang Sudah Ada:
```rust
#[async_trait]
pub trait ChatContract: Send + Sync {
    async fn send_message(...) -> Result<ChatMessageDto, ContractError>;
    async fn get_messages(...) -> Result<Vec<ChatMessageDto>, ContractError>;
    async fn get_or_create_buyer_room(...) -> Result<ChatRoomDto, ContractError>;
    async fn list_active_rooms(...) -> Result<Vec<ChatRoomDto>, ContractError>;
}
```

---

## ✅ Acceptance Criteria

### Step 1: Buat Crate Module Baru `crates/modules/chat`

```
crates/modules/chat/
├── Cargo.toml
└── src/
    └── lib.rs
```

**`Cargo.toml`:**
```toml
[package]
name = "program1-module-chat"
version.workspace = true
edition.workspace = true

[dependencies]
program1-contracts = { path = "../../contracts" }
program1-core = { path = "../../core" }
async-trait.workspace = true
sqlx.workspace = true
uuid.workspace = true
chrono.workspace = true
tracing.workspace = true
```

### Step 2: Buat Migration SQL `010_create_chat_tables.sql`

```sql
-- Chat rooms (1 room per buyer)
CREATE TABLE IF NOT EXISTS chat_rooms (
    id TEXT PRIMARY KEY,
    buyer_id TEXT NOT NULL,
    buyer_name TEXT NOT NULL DEFAULT '',
    last_message TEXT,
    unread_count INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(buyer_id)
);

-- Chat messages
CREATE TABLE IF NOT EXISTS chat_messages (
    id TEXT PRIMARY KEY,
    room_id TEXT NOT NULL REFERENCES chat_rooms(id),
    sender_type TEXT NOT NULL CHECK(sender_type IN ('Buyer','Seller','System')),
    sender_id TEXT NOT NULL,
    sender_name TEXT NOT NULL DEFAULT '',
    content TEXT NOT NULL,
    is_read INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_chat_messages_room ON chat_messages(room_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_chat_rooms_buyer ON chat_rooms(buyer_id);
```

### Step 3: Implement `ChatContract` di `lib.rs`

Fungsi-fungsi yang perlu diimplementasikan:

| Method | Deskripsi |
|--------|-----------|
| `send_message(room_id, sender_type, sender_id, sender_name, content)` | INSERT message ke DB, update `last_message` dan `unread_count` di room |
| `get_messages(room_id, limit)` | SELECT pesan terbaru dengan LIMIT, ordered by `created_at DESC` |
| `get_or_create_buyer_room(buyer_id, buyer_name)` | SELECT existing room atau INSERT baru jika belum ada |
| `list_active_rooms()` | SELECT semua room ordered by `updated_at DESC` (untuk admin panel seller) |

### Step 4: Tambahkan ke Workspace

1. Tambahkan `"crates/modules/chat"` ke `[workspace.members]` di `Cargo.toml` root
2. Tambahkan dependency `program1-module-chat` di `crates/web/Cargo.toml`
3. Inisialisasi `ChatModule` di `crates/web/src/main.rs`
4. Tambahkan `chat_contract: Arc<dyn ChatContract>` ke `AppState`

### Step 5: Buat API Endpoints di `crates/web/src/handlers/`

Buat file baru `crates/web/src/handlers/chat.rs`:

| Method | Path | Auth | Deskripsi |
|--------|------|------|-----------|
| `POST` | `/api/v1/chat/rooms` | Buyer JWT | Get or create chat room for logged-in buyer |
| `GET` | `/api/v1/chat/rooms/:room_id/messages` | Buyer/Seller JWT | Get messages (limit=50) |
| `POST` | `/api/v1/chat/rooms/:room_id/messages` | Buyer/Seller JWT | Send new message |
| `GET` | `/api/v1/admin/chat/rooms` | Seller JWT | List all active chat rooms (admin panel) |
| `POST` | `/api/v1/admin/chat/rooms/:room_id/messages` | Seller JWT | Seller reply to buyer |

### Step 6: Unit Tests

Minimal tests:
1. Test `send_message` — simpan dan bisa retrieve
2. Test `get_or_create_buyer_room` — idempotent (panggil 2x, hanya 1 room tercipta)
3. Test `list_active_rooms` — returned ordered by `updated_at`
4. Test `get_messages` — respects `limit` parameter

---

## 📁 File Yang Harus Dibuat/Diubah

| Action | File |
|--------|------|
| **CREATE** | `crates/modules/chat/Cargo.toml` |
| **CREATE** | `crates/modules/chat/src/lib.rs` |
| **CREATE** | `migrations/sqlite/010_create_chat_tables.sql` |
| **CREATE** | `crates/web/src/handlers/chat.rs` |
| **MODIFY** | `Cargo.toml` (root workspace members) |
| **MODIFY** | `crates/web/Cargo.toml` (add chat dependency) |
| **MODIFY** | `crates/web/src/main.rs` (init ChatModule) |
| **MODIFY** | `crates/web/src/state.rs` (add `chat_contract` field) |
| **MODIFY** | `crates/web/src/handlers/mod.rs` (export chat handlers) |
| **MODIFY** | `crates/web/src/routes.rs` (register chat routes) |

---

## ⚠️ Catatan Penting

- **Jangan implement WebSocket dulu** — cukup HTTP polling. WebSocket bisa ditambahkan nanti.
- Gunakan pattern yang sama seperti `AuditModule` — simpan data ke SQLite via `sqlx`.
- `sender_type` disimpan sebagai TEXT di SQLite, konversi di Rust.
- Pastikan `cargo test --workspace` pass sebelum push.
