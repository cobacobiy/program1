# Issue #19 — Frontend Modularization (store.js Refactor)

> **Prioritas**: 🟢 MEDIUM
> **Estimasi**: 2-3 hari
> **Depends On**: —
> **Skill Level**: Junior Frontend Developer

---

## 🔍 Masalah Saat Ini

File frontend monolithic sangat besar:

| File | Lines | Bytes |
|------|-------|-------|
| `store.js` | 1,426 | 50KB |
| `store.html` | 523 | 27KB |
| `store.css` | 1,402 | 28KB |
| `app.js` | 1,607 | 64KB |
| `index.html` | 913 | 47KB |
| `style.css` | 1,146 | 22KB |

**Total: 7,017 lines** dalam 6 file saja. Ini membuat:
- ❌ Sulit di-maintain dan di-debug
- ❌ Junior developer takut menyentuh file 1600 baris
- ❌ Git merge conflict sering terjadi karena semua orang edit file yang sama
- ❌ Tidak bisa lazy-load komponen

---

## ✅ Acceptance Criteria

### Step 1: Pecah `store.js` Menjadi Module Files

Buat directory structure baru:
```
crates/web/static/
├── store.html                    ← Keep (reduced, loads modules)
├── store.css                     ← Keep
├── store/
│   ├── store-main.js             ← Init, DOMContentLoaded, global state
│   ├── store-auth.js             ← Login, register, Google OAuth, OTP
│   ├── store-catalog.js          ← Product listing, category filter, search
│   ├── store-cart.js             ← Cart logic, add/remove/update quantity
│   ├── store-checkout.js         ← Checkout flow, address selection
│   ├── store-chat.js             ← In-app chat, WhatsApp redirect
│   └── store-toast.js            ← Toast notification system
├── index.html                    ← Keep (reduced, loads modules)
├── style.css                     ← Keep
├── admin/
│   ├── admin-main.js             ← Init, navigation, global state
│   ├── admin-users.js            ← User management CRUD
│   ├── admin-catalog.js          ← Catalog management CRUD
│   ├── admin-inventory.js        ← Inventory/stock management
│   ├── admin-orders.js           ← Order management & status
│   ├── admin-analytics.js        ← Dashboard analytics & charts
│   ├── admin-audit.js            ← Audit log viewer
│   └── admin-buyers.js           ← Customer directory (CRM)
```

### Step 2: Extract Functions by Responsibility

**`store-toast.js`** — Extract from store.js:
```javascript
// showToast(), window.alert override
```

**`store-auth.js`** — Extract:
```javascript
// handleBuyerLogin(), handleBuyerRegister(), handleGoogleAuth()
// requestOtp(), verifyOtp(), renderOtpModal()
// initGoogleOneTap()
```

**`store-catalog.js`** — Extract:
```javascript
// loadCatalog(), renderProducts(), filterByCategory()
// searchProducts(), renderCategoryTabs()
```

**`store-cart.js`** — Extract:
```javascript
// addToCart(), removeFromCart(), updateCartQuantity()
// renderCartSidebar(), calculateCartTotal()
```

**`store-checkout.js`** — Extract:
```javascript
// handleCheckout(), loadAddresses(), renderAddressSelector()
// createOrder()
```

**`store-chat.js`** — Extract:
```javascript
// openInAppChatWindow(), handleDirectWhatsAppChat()
// renderChatWidget(), sendChatMessage()
```

### Step 3: Load Modules via `<script>` Tags

Di `store.html`:
```html
<!-- Module scripts (order matters for dependencies) -->
<script src="/assets/store/store-toast.js"></script>
<script src="/assets/store/store-auth.js"></script>
<script src="/assets/store/store-catalog.js"></script>
<script src="/assets/store/store-cart.js"></script>
<script src="/assets/store/store-checkout.js"></script>
<script src="/assets/store/store-chat.js"></script>
<script src="/assets/store/store-main.js"></script>
```

### Step 4: Shared State Pattern

Buat global state object di `store-main.js`:
```javascript
const StoreState = {
    cart: [],
    catalog: [],
    activeCategory: 'ALL',
    searchQuery: '',
    buyerToken: localStorage.getItem('program1_buyer_token') || null,
    activeBuyer: null,
    buyerAddresses: [],
    selectedAddressId: null,
    storeWhatsAppNumber: '',
    googleClientId: null,
};
```

Semua module lain access via `StoreState.cart`, `StoreState.buyerToken`, etc.

### Step 5: Testing Verification

Karena ini refactor tanpa fitur baru, verifikasi dengan:
1. Semua fitur storefront tetap berjalan (login, catalog, cart, checkout, chat)
2. Semua fitur admin tetap berjalan (users, catalog, inventory, orders, analytics, audit)
3. Browser console tidak ada error
4. Responsif di mobile dan desktop

---

## 📁 File Yang Harus Dibuat/Diubah

| Action | File |
|--------|------|
| **CREATE** | `crates/web/static/store/store-main.js` |
| **CREATE** | `crates/web/static/store/store-auth.js` |
| **CREATE** | `crates/web/static/store/store-catalog.js` |
| **CREATE** | `crates/web/static/store/store-cart.js` |
| **CREATE** | `crates/web/static/store/store-checkout.js` |
| **CREATE** | `crates/web/static/store/store-chat.js` |
| **CREATE** | `crates/web/static/store/store-toast.js` |
| **CREATE** | `crates/web/static/admin/admin-main.js` |
| **CREATE** | `crates/web/static/admin/admin-users.js` |
| **CREATE** | `crates/web/static/admin/admin-catalog.js` |
| **CREATE** | `crates/web/static/admin/admin-inventory.js` |
| **CREATE** | `crates/web/static/admin/admin-orders.js` |
| **CREATE** | `crates/web/static/admin/admin-analytics.js` |
| **CREATE** | `crates/web/static/admin/admin-audit.js` |
| **CREATE** | `crates/web/static/admin/admin-buyers.js` |
| **MODIFY** | `crates/web/static/store.html` (replace single script with module scripts) |
| **MODIFY** | `crates/web/static/index.html` (replace single script with module scripts) |
| **DELETE** | `crates/web/static/store.js` (setelah semua logic dipindahkan) |
| **DELETE** | `crates/web/static/app.js` (setelah semua logic dipindahkan) |

---

## ⚠️ Catatan Penting

- **Ini murni refactor** — TIDAK ada fitur baru. Semua behavior harus identik.
- Gunakan `<script>` tags biasa (bukan ES modules) untuk kompatibilitas browser lama
- Global state shared via `StoreState` / `AdminState` object
- Test di mobile (Chrome DevTools device emulator) setelah refactor
- **Ikuti aturan AGENTS.md**: No monolithic HTML — modularize assets ke separate files
