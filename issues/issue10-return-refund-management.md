# Issue 10: Return/Refund Management System

> **Prioritas:** 🔴 CRITICAL  
> **Estimasi:** 4-5 hari  
> **Kesulitan:** ⭐⭐⭐⭐ Sulit  
> **Prerequisite:** Order Status Workflow (sudah DONE), Payment Gateway (sudah DONE)

---

## 📋 Deskripsi

Setiap toko online **wajib** punya sistem pengembalian barang (return) dan pengembalian dana (refund).  
Issue ini menambahkan:
- Buyer bisa ajukan return/refund untuk order yang sudah delivered
- Admin review request dan approve/reject
- Refund terintegrasi dengan Midtrans (jika payment sudah settled)
- Status tracking untuk return process

---

## 🎯 Acceptance Criteria

- [ ] Buyer bisa ajukan return request (dengan alasan + foto bukti)
- [ ] Admin bisa lihat, approve, atau reject return request
- [ ] Return status: `pending → approved → return_shipped → received → refunded` atau `rejected`
- [ ] Jika approved, stok produk bertambah kembali (inventory restock)
- [ ] Refund amount dihitung otomatis
- [ ] Unit test minimal 4 test case

---

## 📐 Langkah-Langkah

### Langkah 1: Buat Migration

**File:** `migrations/sqlite/013_create_returns.sql`

```sql
CREATE TABLE IF NOT EXISTS return_requests (
    id TEXT PRIMARY KEY NOT NULL,
    order_id TEXT NOT NULL REFERENCES orders(id),
    buyer_id TEXT NOT NULL,
    reason TEXT NOT NULL,              -- 'defective', 'wrong_item', 'not_as_described', 'other'
    description TEXT,
    evidence_urls TEXT,                -- JSON array of image URLs
    status TEXT NOT NULL DEFAULT 'pending', -- pending, approved, rejected, return_shipped, received, refunded
    refund_amount_cents INTEGER NOT NULL,
    admin_notes TEXT,
    processed_by TEXT,                 -- admin user_id who processed
    processed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_returns_order ON return_requests(order_id);
CREATE INDEX idx_returns_buyer ON return_requests(buyer_id);
CREATE INDEX idx_returns_status ON return_requests(status);
```

### Langkah 2: Tambah DTO di Contracts

**File:** `crates/contracts/src/lib.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReturnRequest {
    pub id: String,
    pub order_id: String,
    pub buyer_id: String,
    pub reason: String,
    pub description: Option<String>,
    pub evidence_urls: Vec<String>,
    pub status: String,
    pub refund_amount_cents: i64,
    pub admin_notes: Option<String>,
    pub processed_by: Option<String>,
    pub processed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateReturnRequest {
    #[validate(length(min = 1))]
    pub order_id: String,
    #[validate(length(min = 1))]
    pub reason: String,
    #[validate(length(max = 1000))]
    pub description: Option<String>,
    pub evidence_urls: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct ProcessReturnRequest {
    #[validate(length(min = 1))]
    pub action: String,         // "approve" or "reject"
    pub admin_notes: Option<String>,
    pub refund_amount_override: Option<i64>,  // optional override
}
```

### Langkah 3: Buat ReturnContract Trait

**File:** `crates/contracts/src/lib.rs`

```rust
#[async_trait]
pub trait ReturnContract: Send + Sync {
    /// Buyer: Create return request
    async fn create_return_request(
        &self, buyer_id: &str, req: CreateReturnRequest,
    ) -> Result<ReturnRequest, ContractError>;
    
    /// Buyer: List my return requests
    async fn list_buyer_returns(
        &self, buyer_id: &str,
    ) -> Result<Vec<ReturnRequest>, ContractError>;
    
    /// Admin: List all return requests (with filters)
    async fn list_all_returns(
        &self, status_filter: Option<&str>,
    ) -> Result<Vec<ReturnRequest>, ContractError>;
    
    /// Admin: Process return (approve/reject)
    async fn process_return(
        &self, return_id: &str, admin_id: &str, req: ProcessReturnRequest,
    ) -> Result<ReturnRequest, ContractError>;
    
    /// Admin: Update return status (e.g. received, refunded)
    async fn update_return_status(
        &self, return_id: &str, new_status: &str,
    ) -> Result<ReturnRequest, ContractError>;
}
```

### Langkah 4: Buat Return Module

```
crates/modules/return/
├── Cargo.toml
└── src/
    └── lib.rs
```

#### Logika Penting di `create_return_request`:

```rust
async fn create_return_request(
    &self, buyer_id: &str, req: CreateReturnRequest,
) -> Result<ReturnRequest, ContractError> {
    // 1. Verify order exists and belongs to buyer
    let order = /* fetch order */;
    if order.buyer_id != buyer_id {
        return Err(ContractError::Unauthorized("Not your order".into()));
    }
    
    // 2. Verify order status is "delivered"
    if order.status != "delivered" {
        return Err(ContractError::ValidationError(
            "Can only return delivered orders".into()
        ));
    }
    
    // 3. Check no existing active return for this order
    let existing = /* query existing return for order_id where status != 'rejected' */;
    if existing.is_some() {
        return Err(ContractError::AlreadyExists(
            "Return request already exists for this order".into()
        ));
    }
    
    // 4. Calculate refund amount (= order total)
    let refund_amount = order.total_cents;
    
    // 5. Validate reason is one of allowed values
    let valid_reasons = ["defective", "wrong_item", "not_as_described", "other"];
    if !valid_reasons.contains(&req.reason.as_str()) {
        return Err(ContractError::ValidationError("Invalid return reason".into()));
    }
    
    // 6. Insert return_requests record
    // 7. Return the created record
}
```

#### Logika Penting di `process_return`:

```rust
async fn process_return(
    &self, return_id: &str, admin_id: &str, req: ProcessReturnRequest,
) -> Result<ReturnRequest, ContractError> {
    let return_req = /* fetch return request */;
    
    if return_req.status != "pending" {
        return Err(ContractError::ValidationError(
            "Can only process pending returns".into()
        ));
    }
    
    match req.action.as_str() {
        "approve" => {
            // Update status to "approved"
            // If refund_amount_override, update refund amount
        },
        "reject" => {
            // Update status to "rejected"
        },
        _ => return Err(ContractError::ValidationError("Invalid action".into())),
    }
    
    // Update processed_by, processed_at, admin_notes
}
```

### Langkah 5: Register Module

- Tambahkan ke workspace `Cargo.toml` members
- Tambahkan ke `AppState`
- Init di `main.rs`

### Langkah 6: Tambah Handlers & Routes

**File:** `crates/web/src/handlers/returns.rs` (file baru)

Buyer routes:
```rust
// POST /api/v1/buyer/returns          — create return request
// GET  /api/v1/buyer/returns          — list my returns
```

Admin routes:
```rust
// GET   /api/v1/admin/returns         — list all returns
// PATCH /api/v1/admin/returns/:id     — process (approve/reject)
// PATCH /api/v1/admin/returns/:id/status — update status
```

### Langkah 7: Buyer Frontend — Return Request Form

**File:** `crates/web/static/store/store-checkout.js`

Di order detail (setelah delivered), tambahkan tombol "Ajukan Pengembalian":

```javascript
function renderReturnButton(order) {
    if (order.status !== 'delivered') return '';
    return `<button onclick="showReturnForm('${order.id}')" class="btn-return">
        🔄 Ajukan Pengembalian
    </button>`;
}

function showReturnForm(orderId) {
    const modal = document.getElementById('returnModal');
    modal.innerHTML = `
        <h3>Ajukan Pengembalian</h3>
        <select id="returnReason">
            <option value="defective">Barang Cacat/Rusak</option>
            <option value="wrong_item">Barang Salah</option>
            <option value="not_as_described">Tidak Sesuai Deskripsi</option>
            <option value="other">Lainnya</option>
        </select>
        <textarea id="returnDescription" placeholder="Jelaskan alasan pengembalian..."></textarea>
        <div class="return-evidence">
            <label>Upload Foto Bukti (opsional):</label>
            <input type="file" id="returnEvidence" accept="image/*" multiple />
        </div>
        <button onclick="submitReturn('${orderId}')">Kirim Permintaan</button>
    `;
    modal.classList.add('show');
}
```

### Langkah 8: Admin Frontend — Return Management

**File:** `crates/web/static/admin/admin-orders.js`

Tambahkan tab/section untuk manage returns:

```javascript
async function loadReturnRequests() {
    const res = await fetch('/api/v1/admin/returns', {
        headers: { 'Authorization': `Bearer ${token}` }
    });
    const returns = await res.json();
    // Render table with approve/reject buttons
}
```

### Langkah 9: (Advanced) Inventory Restock on Return

Saat return status = "received", otomatis tambah kembali stok:

```rust
// Di update_return_status, setelah status = "received":
if new_status == "received" {
    // Get order items
    // For each item, increment inventory stock
    // self.inventory_contract.adjust_stock(product_id, +quantity, "Return restock").await?;
}
```

### Langkah 10: Tulis Unit Test

**File:** `crates/web/tests/return_test.rs`

```rust
#[tokio::test]
async fn test_create_return_for_delivered_order() { /* ... */ }

#[tokio::test]
async fn test_cannot_return_non_delivered_order() { /* ... */ }

#[tokio::test]
async fn test_admin_approve_return() { /* ... */ }

#[tokio::test]
async fn test_admin_reject_return() { /* ... */ }

#[tokio::test]
async fn test_duplicate_return_rejected() { /* ... */ }
```

### Langkah 11: Verifikasi

```bash
cargo test --workspace
cargo check --workspace
```

---

## ⚠️ Perhatian

- Return hanya bisa diajukan untuk order **delivered** — bukan pending/processing/shipped
- 1 order = maksimal 1 return request aktif (yang tidak rejected)
- Buyer hanya bisa lihat return miliknya sendiri (ownership check!)
- Admin notes wajib saat reject (beri alasan ke buyer)
- Foto bukti upload via existing `/api/v1/uploads/images` endpoint
- Saat approve refund, jangan langsung potong saldo — proses lewat Midtrans refund API
