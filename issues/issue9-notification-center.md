# Issue 9: Notification Center (In-App Notifications)

> **Prioritas:** 🟡 HIGH  
> **Estimasi:** 3-4 hari  
> **Kesulitan:** ⭐⭐⭐ Menengah  
> **Prerequisite:** Tidak ada — bisa dikerjakan independen

---

## 📋 Deskripsi

Saat ini buyer dan admin tidak punya **notifikasi dalam aplikasi**.  
Issue ini menambahkan sistem notification center supaya:
- Buyer mendapat notifikasi saat order berubah status, chat baru, promo
- Admin mendapat notifikasi saat ada order baru, low stock alert, chat baru
- Notifikasi ditampilkan sebagai badge + dropdown di navbar

---

## 🎯 Acceptance Criteria

- [ ] Tabel `notifications` di database untuk simpan notifikasi
- [ ] Badge counter di navbar (🔔 3) menunjukkan jumlah notif belum dibaca
- [ ] Klik bell → dropdown list notifikasi terbaru
- [ ] Buyer bisa mark as read / mark all as read
- [ ] Admin punya notification center sendiri
- [ ] Notifikasi otomatis dibuat saat order status berubah
- [ ] Unit test minimal 2 test case

---

## 📐 Langkah-Langkah

### Langkah 1: Buat Migration

**File:** `migrations/sqlite/013_create_notifications.sql`

```sql
CREATE TABLE IF NOT EXISTS notifications (
    id TEXT PRIMARY KEY NOT NULL,
    recipient_type TEXT NOT NULL,     -- 'buyer' or 'seller'
    recipient_id TEXT NOT NULL,       -- buyer_id or user_id (seller)
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    notification_type TEXT NOT NULL,  -- 'order_status', 'new_order', 'low_stock', 'chat', 'promo'
    reference_id TEXT,               -- order_id, product_id, chat_room_id, etc.
    reference_type TEXT,             -- 'order', 'product', 'chat', etc.
    is_read INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_notifications_recipient ON notifications(recipient_type, recipient_id, is_read);
CREATE INDEX idx_notifications_created ON notifications(created_at DESC);
```

### Langkah 2: Tambah DTO di Contracts

**File:** `crates/contracts/src/lib.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Notification {
    pub id: String,
    pub recipient_type: String,
    pub recipient_id: String,
    pub title: String,
    pub message: String,
    pub notification_type: String,
    pub reference_id: Option<String>,
    pub reference_type: Option<String>,
    pub is_read: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NotificationCount {
    pub unread: i64,
    pub total: i64,
}
```

### Langkah 3: Buat NotificationContract Trait

**File:** `crates/contracts/src/lib.rs`

```rust
#[async_trait]
pub trait NotificationContract: Send + Sync {
    /// Create a new notification
    async fn create_notification(
        &self,
        recipient_type: &str,
        recipient_id: &str,
        title: &str,
        message: &str,
        notification_type: &str,
        reference_id: Option<&str>,
        reference_type: Option<&str>,
    ) -> Result<Notification, ContractError>;

    /// List notifications for a recipient (paginated)
    async fn list_notifications(
        &self,
        recipient_type: &str,
        recipient_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Notification>, ContractError>;

    /// Get unread count
    async fn get_unread_count(
        &self,
        recipient_type: &str,
        recipient_id: &str,
    ) -> Result<NotificationCount, ContractError>;

    /// Mark single notification as read
    async fn mark_as_read(&self, notification_id: &str, recipient_id: &str) -> Result<(), ContractError>;

    /// Mark all notifications as read
    async fn mark_all_as_read(&self, recipient_type: &str, recipient_id: &str) -> Result<(), ContractError>;
}
```

### Langkah 4: Buat Notification Module

```
crates/modules/notification/
├── Cargo.toml
└── src/
    └── lib.rs
```

**File:** `crates/modules/notification/Cargo.toml`

```toml
[package]
name = "program1-module-notification"
version.workspace = true
edition.workspace = true

[dependencies]
program1-contracts = { path = "../../contracts" }
sqlx.workspace = true
async-trait.workspace = true
uuid.workspace = true
tracing.workspace = true
```

Implementasikan `NotificationContract` di `lib.rs`.

### Langkah 5: Integrate dengan Order Status Changes

**File:** `crates/modules/order/src/lib.rs`

Saat order status berubah, buat notifikasi:

```rust
// Setelah update_order_status berhasil:
if let Some(notification_contract) = &self.notification_contract {
    let (title, message) = match new_status.as_str() {
        "processing" => ("Pesanan Diproses", "Pesanan Anda sedang diproses oleh penjual"),
        "shipped" => ("Pesanan Dikirim", &format!("Pesanan Anda telah dikirim. Resi: {}", tracking)),
        "delivered" => ("Pesanan Diterima", "Pesanan Anda telah sampai. Jangan lupa beri ulasan!"),
        "cancelled" => ("Pesanan Dibatalkan", "Pesanan Anda telah dibatalkan"),
        _ => ("Update Pesanan", &format!("Status pesanan: {}", new_status)),
    };
    
    let _ = notification_contract.create_notification(
        "buyer", &order.buyer_id, title, message,
        "order_status", Some(&order.id), Some("order"),
    ).await;
}
```

### Langkah 6: Tambah Handlers & Routes

**File:** `crates/web/src/handlers/notification.rs` (file baru)

```rust
/// GET /api/v1/buyer/notifications
pub async fn buyer_list_notifications_handler(/* ... */) { /* ... */ }

/// GET /api/v1/buyer/notifications/count
pub async fn buyer_notification_count_handler(/* ... */) { /* ... */ }

/// POST /api/v1/buyer/notifications/:id/read
pub async fn buyer_mark_read_handler(/* ... */) { /* ... */ }

/// POST /api/v1/buyer/notifications/read-all
pub async fn buyer_mark_all_read_handler(/* ... */) { /* ... */ }
```

### Langkah 7: Register Routes

**File:** `crates/web/src/routes.rs`

Buyer routes:
```rust
.route("/api/v1/buyer/notifications", get(buyer_list_notifications_handler))
.route("/api/v1/buyer/notifications/count", get(buyer_notification_count_handler))
.route("/api/v1/buyer/notifications/:id/read", post(buyer_mark_read_handler))
.route("/api/v1/buyer/notifications/read-all", post(buyer_mark_all_read_handler))
```

### Langkah 8: Frontend — Notification Bell Component

**File:** `crates/web/static/store/store-notification.js` (file baru)

```javascript
// Poll unread count every 30 seconds
let notifPollInterval = null;

async function initNotifications() {
    if (!window.StoreState?.buyerToken) return;
    await updateNotifBadge();
    notifPollInterval = setInterval(updateNotifBadge, 30000);
}

async function updateNotifBadge() {
    try {
        const res = await fetch('/api/v1/buyer/notifications/count', {
            headers: { 'Authorization': `Bearer ${window.StoreState.buyerToken}` }
        });
        const data = await res.json();
        const badge = document.getElementById('notifBadge');
        if (badge) {
            badge.textContent = data.unread;
            badge.style.display = data.unread > 0 ? 'flex' : 'none';
        }
    } catch (e) { /* silent fail */ }
}

async function showNotifDropdown() {
    const res = await fetch('/api/v1/buyer/notifications?limit=10', {
        headers: { 'Authorization': `Bearer ${window.StoreState.buyerToken}` }
    });
    const notifs = await res.json();
    
    const dropdown = document.getElementById('notifDropdown');
    dropdown.innerHTML = notifs.length === 0
        ? '<div class="notif-empty">Tidak ada notifikasi</div>'
        : notifs.map(n => `
            <div class="notif-item ${n.is_read ? '' : 'unread'}" onclick="openNotif('${n.id}', '${n.reference_type}', '${n.reference_id}')">
                <div class="notif-title">${n.title}</div>
                <div class="notif-msg">${n.message}</div>
                <div class="notif-time">${timeAgo(n.created_at)}</div>
            </div>
        `).join('') + '<div class="notif-footer"><a onclick="markAllRead()">Tandai semua dibaca</a></div>';
    
    dropdown.classList.toggle('show');
}
```

### Langkah 9: Tambah Bell Icon ke Navbar

**File:** `crates/web/static/store.html`

```html
<div class="notif-wrapper" onclick="showNotifDropdown()">
    🔔
    <span class="notif-badge" id="notifBadge" style="display:none">0</span>
    <div class="notif-dropdown" id="notifDropdown"></div>
</div>
```

### Langkah 10: Tambah CSS

**File:** `crates/web/static/store.css`

```css
.notif-wrapper { position: relative; cursor: pointer; }
.notif-badge {
    position: absolute; top: -6px; right: -6px;
    background: #e74c3c; color: white;
    border-radius: 50%; width: 18px; height: 18px;
    font-size: 0.7rem; display: flex; align-items: center; justify-content: center;
}
.notif-dropdown {
    display: none; position: absolute; right: 0; top: 100%;
    width: 320px; max-height: 400px; overflow-y: auto;
    background: white; border-radius: 12px; box-shadow: 0 8px 24px rgba(0,0,0,0.15);
    z-index: 1000;
}
.notif-dropdown.show { display: block; }
.notif-item { padding: 12px 16px; border-bottom: 1px solid #f0f0f0; cursor: pointer; }
.notif-item.unread { background: #f0f7ff; }
.notif-item:hover { background: #e8f0fe; }
```

### Langkah 11: Register Module di Workspace

**File:** `Cargo.toml` (root) — tambahkan `"crates/modules/notification"`

**File:** `crates/web/src/state.rs` — tambahkan `notification_contract`

**File:** `crates/web/src/main.rs` — init `NotificationModule`

### Langkah 12: Tulis Unit Test

**File:** `crates/web/tests/notification_test.rs`

```rust
#[tokio::test]
async fn test_create_and_list_notifications() {
    // 1. Create 3 notifications for buyer
    // 2. List → expect 3
    // 3. Get count → unread = 3
}

#[tokio::test]
async fn test_mark_read_and_count() {
    // 1. Create 2 notifications
    // 2. Mark 1 as read
    // 3. Count → unread = 1
    // 4. Mark all as read → unread = 0
}
```

### Langkah 13: Verifikasi

```bash
cargo test --workspace
cargo check --workspace
```

---

## ⚠️ Perhatian

- Notification polling setiap 30 detik — jangan terlalu sering (beban server)
- Buyer hanya bisa baca notifikasi **miliknya sendiri** (ownership check!)
- `recipient_id` harus match dengan JWT claims
- Cleanup: pertimbangkan auto-delete notifikasi > 90 hari (optional)
- Jangan kirim notifikasi untuk action yang dilakukan oleh user sendiri
