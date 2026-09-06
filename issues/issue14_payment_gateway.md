# Issue #14 — Payment Gateway Integration (Midtrans)

> **Prioritas**: 🔴 CRITICAL
> **Estimasi**: 4-5 hari
> **Depends On**: —
> **Skill Level**: Mid Rust Developer

---

## 🔍 Masalah Saat Ini

Saat ini **tidak ada payment processing sama sekali**. Order bisa dibuat tapi:
- ❌ Tidak ada pembayaran — buyer tidak bisa bayar
- ❌ Status order langsung `pending` tanpa follow-up
- ❌ Tidak ada integrasi ke payment gateway manapun (Midtrans/Xendit/Stripe)

### Storefront Flow Saat Ini:
```
Buyer → Checkout → Order Created (status: "pending") → ... ??? (tidak ada lanjutannya)
```

### Storefront Flow Yang Diharapkan:
```
Buyer → Checkout → Midtrans Snap Payment Page → Payment Callback → Order Updated (status: "paid")
```

---

## ✅ Acceptance Criteria

### Step 1: Definisikan `PaymentContract` di `crates/contracts/src/lib.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaymentTransactionDto {
    pub id: Uuid,
    pub order_id: Uuid,
    pub payment_method: String,       // "bank_transfer", "gopay", "qris", etc.
    pub amount: f64,
    pub currency: String,             // "IDR"
    pub status: String,               // "pending", "paid", "expired", "refunded"
    pub provider_ref: Option<String>, // Midtrans transaction_id
    pub snap_token: Option<String>,   // Midtrans Snap Token for frontend
    pub snap_redirect_url: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreatePaymentRequest {
    pub order_id: Uuid,
}

#[async_trait]
pub trait PaymentContract: Send + Sync {
    /// Create payment intent — calls Midtrans Snap API, returns snap_token
    async fn create_payment(
        &self,
        order_id: Uuid,
        amount: f64,
        customer_name: &str,
        customer_email: &str,
    ) -> Result<PaymentTransactionDto, ContractError>;

    /// Handle webhook notification from Midtrans
    async fn handle_notification(
        &self,
        payload: serde_json::Value,
    ) -> Result<PaymentTransactionDto, ContractError>;

    /// Get payment status by order_id
    async fn get_payment_by_order(
        &self,
        order_id: Uuid,
    ) -> Result<Option<PaymentTransactionDto>, ContractError>;
}
```

### Step 2: Buat Crate Module `crates/modules/payment`

```
crates/modules/payment/
├── Cargo.toml
└── src/
    └── lib.rs
```

Tambahan dependency:
```toml
[dependencies]
reqwest = { version = "0.12", features = ["json"] }
base64 = "0.22"
```

### Step 3: Buat Migration `011_create_payment_transactions.sql`

```sql
CREATE TABLE IF NOT EXISTS payment_transactions (
    id TEXT PRIMARY KEY,
    order_id TEXT NOT NULL,
    payment_method TEXT NOT NULL DEFAULT '',
    amount REAL NOT NULL,
    currency TEXT NOT NULL DEFAULT 'IDR',
    status TEXT NOT NULL DEFAULT 'pending',
    provider_ref TEXT,
    snap_token TEXT,
    snap_redirect_url TEXT,
    paid_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(order_id)
);

CREATE INDEX IF NOT EXISTS idx_payment_order ON payment_transactions(order_id);
CREATE INDEX IF NOT EXISTS idx_payment_status ON payment_transactions(status);
```

### Step 4: Implementasi Midtrans Snap API

**Midtrans Snap Token Request:**
```
POST https://app.sandbox.midtrans.com/snap/v1/transactions
Authorization: Basic base64(SERVER_KEY:)
Content-Type: application/json

{
  "transaction_details": {
    "order_id": "ORDER-xxx",
    "gross_amount": 150000
  },
  "customer_details": {
    "first_name": "John",
    "email": "john@example.com"
  }
}
```

Response → `{ "token": "snap-token-xxx", "redirect_url": "https://..." }`

**Environment Variables baru (tambahkan ke `.env.example`):**
```env
# --- Payment Gateway (Midtrans) ---
MIDTRANS_SERVER_KEY=SB-Mid-server-xxx
MIDTRANS_CLIENT_KEY=SB-Mid-client-xxx
MIDTRANS_IS_PRODUCTION=false
```

### Step 5: API Endpoints

| Method | Path | Auth | Deskripsi |
|--------|------|------|-----------|
| `POST` | `/api/v1/payments` | Buyer JWT | Create payment → returns snap_token |
| `POST` | `/api/v1/payments/notification` | Public (webhook) | Midtrans callback notification |
| `GET` | `/api/v1/payments/order/:order_id` | Buyer/Seller JWT | Check payment status |

### Step 6: Frontend Integration di `store.js`

Setelah order berhasil dibuat, tampilkan Midtrans Snap popup:
```javascript
// Load Midtrans Snap.js
// <script src="https://app.sandbox.midtrans.com/snap/snap.js" data-client-key="CLIENT_KEY"></script>

async function payOrder(orderId) {
    const res = await fetch('/api/v1/payments', {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${buyerToken}`, 'Content-Type': 'application/json' },
        body: JSON.stringify({ order_id: orderId })
    });
    const data = await res.json();
    snap.pay(data.snap_token, {
        onSuccess: () => showToast('Pembayaran berhasil!', 'success'),
        onPending: () => showToast('Menunggu pembayaran...', 'warning'),
        onError: () => showToast('Pembayaran gagal', 'error')
    });
}
```

### Step 7: Unit Tests

1. Test create payment — generates snap_token (mock HTTP call)
2. Test handle notification — updates payment status to "paid"
3. Test get payment by order — returns correct payment
4. Test duplicate payment for same order — returns error

---

## 📁 File Yang Harus Dibuat/Diubah

| Action | File |
|--------|------|
| **MODIFY** | `crates/contracts/src/lib.rs` (add PaymentContract) |
| **CREATE** | `crates/modules/payment/Cargo.toml` |
| **CREATE** | `crates/modules/payment/src/lib.rs` |
| **CREATE** | `migrations/sqlite/011_create_payment_transactions.sql` |
| **CREATE** | `crates/web/src/handlers/payment.rs` |
| **MODIFY** | `Cargo.toml` (add workspace member) |
| **MODIFY** | `crates/web/Cargo.toml` |
| **MODIFY** | `crates/web/src/main.rs` |
| **MODIFY** | `crates/web/src/state.rs` |
| **MODIFY** | `crates/web/src/handlers/mod.rs` |
| **MODIFY** | `crates/web/src/routes.rs` |
| **MODIFY** | `crates/core/src/config.rs` (add Midtrans config) |
| **MODIFY** | `.env.example` |
| **MODIFY** | `crates/web/static/store.js` (Snap integration) |
| **MODIFY** | `crates/web/static/store.html` (load Snap.js) |

---

## ⚠️ Catatan Penting

- Gunakan **Midtrans Sandbox** selama development (`SB-Mid-server-xxx`)
- Midtrans notification webhook harus PUBLIC endpoint (tanpa JWT auth)
- **VERIFY signature** di webhook handler untuk mencegah fake notification
- Midtrans `SERVER_KEY` adalah **secret** — JANGAN hardcode di source code
- Pastikan `cargo test --workspace` pass sebelum push
