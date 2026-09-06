# Issue #20 — Buyer Order History & Profile Page

> **Prioritas**: 🟡 HIGH
> **Estimasi**: 2 hari
> **Depends On**: —
> **Skill Level**: Junior Frontend Developer

---

## 🔍 Masalah Saat Ini

Buyer yang sudah login **tidak bisa melihat history order** mereka. Saat ini:
- ❌ Tidak ada halaman "Pesanan Saya"
- ❌ Tidak ada endpoint untuk buyer melihat order sendiri
- ❌ Profile buyer hanya tampilkan nama, tidak ada aksi (edit profile, ganti password)
- ❌ Tidak ada halaman address management yang user-friendly

---

## ✅ Acceptance Criteria

### Step 1: Backend — Buyer Order History Endpoint

Tambahkan ke `OrderContract`:
```rust
async fn list_buyer_orders(
    &self,
    buyer_id: Uuid,
) -> Result<Vec<OmniOrderDto>, ContractError>;
```

Implementasi: `SELECT * FROM orders WHERE buyer_id = $1 ORDER BY created_at DESC`

**API Endpoint:**

| Method | Path | Auth | Deskripsi |
|--------|------|------|-----------|
| `GET` | `/api/v1/buyer/orders` | Buyer JWT | List order history milik buyer yang login |
| `GET` | `/api/v1/buyer/orders/:id` | Buyer JWT | Detail order (validasi buyer_id match) |

### Step 2: Backend — Update Buyer Profile Endpoint

Tambahkan ke `BuyerContract`:
```rust
async fn update_buyer_profile(
    &self,
    buyer_id: Uuid,
    full_name: Option<String>,
    avatar_url: Option<String>,
) -> Result<BuyerAccountDto, ContractError>;
```

**API Endpoint:**

| Method | Path | Auth | Deskripsi |
|--------|------|------|-----------|
| `PUT` | `/api/v1/buyer/profile` | Buyer JWT | Update nama / avatar |

### Step 3: Frontend — Storefront Tabs

Tambahkan navigasi tabs di storefront header (untuk logged-in buyer):

```
🏠 Beranda | 📦 Pesanan Saya | 📍 Alamat | 👤 Profil
```

**Tab: Pesanan Saya**
- List order cards: order ID, tanggal, status badge, total, item count
- Klik order → expand detail (items, shipping address, tracking)
- Status badge: pending=kuning, paid=biru, shipped=ungu, delivered=hijau, cancelled=merah

**Tab: Alamat**
- List semua alamat buyer
- Tombol "Tambah Alamat Baru"
- Edit / Hapus / Set Default untuk setiap alamat
- Alamat default ditandai ⭐

**Tab: Profil**
- Tampilkan: nama, email, phone, avatar
- Tombol "Edit Profil" → modal/inline edit
- Status verifikasi telepon ✅ / ❌

### Step 4: UI Components

**Order Status Badge:**
```css
.status-pending { background: #FEF3C7; color: #92400E; }
.status-paid { background: #DBEAFE; color: #1E40AF; }
.status-processing { background: #E0E7FF; color: #3730A3; }
.status-shipped { background: #EDE9FE; color: #5B21B6; }
.status-delivered { background: #D1FAE5; color: #065F46; }
.status-completed { background: #D1FAE5; color: #065F46; }
.status-cancelled { background: #FEE2E2; color: #991B1B; }
```

**Order Card:**
```
┌──────────────────────────────────────────────┐
│ #ORD-20260906-001         [Shipped] 🟣       │
│ 6 Sep 2026                                    │
│ ─────────────────────────────────────────── │
│ 🔸 Kaos Polos Hitam x2          Rp 150.000   │
│ 🔸 Celana Jeans Slim x1         Rp 250.000   │
│ ─────────────────────────────────────────── │
│ Total: Rp 400.000                             │
│ 📦 Tracking: JNT1234567890                    │
└──────────────────────────────────────────────┘
```

### Step 5: Unit Tests

1. Test `list_buyer_orders` — only returns orders for the specific buyer
2. Test `update_buyer_profile` — updates name, returns updated profile
3. Test buyer cannot see other buyer's order → 404 or empty

---

## 📁 File Yang Harus Dibuat/Diubah

| Action | File |
|--------|------|
| **MODIFY** | `crates/contracts/src/lib.rs` (add `list_buyer_orders`, `update_buyer_profile`) |
| **MODIFY** | `crates/modules/order/src/lib.rs` (implement `list_buyer_orders`) |
| **MODIFY** | `crates/modules/buyer/src/lib.rs` (implement `update_buyer_profile`) |
| **MODIFY** | `crates/web/src/handlers/buyer.rs` (add order history + profile update handlers) |
| **MODIFY** | `crates/web/src/handlers/order.rs` (add buyer-scoped order detail) |
| **MODIFY** | `crates/web/src/routes.rs` (register new buyer routes) |
| **MODIFY** | `crates/web/static/store.js` (add order history, profile, address management UI) |
| **MODIFY** | `crates/web/static/store.html` (add tabs navigation) |
| **MODIFY** | `crates/web/static/store.css` (order card + status badge styles) |

---

## ⚠️ Catatan Penting

- **Security**: Buyer hanya bisa lihat order milik sendiri — SELALU filter by `buyer_id` dari JWT
- Jangan expose order milik buyer lain walau buyer tahu `order_id`
- Address management sudah ada endpoint-nya (`/api/v1/buyer/addresses`) — cukup buat UI-nya
- Pastikan `cargo test --workspace` pass sebelum push
