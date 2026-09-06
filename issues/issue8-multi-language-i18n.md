# Issue 8: Multi-Language (i18n) Support — Bahasa Indonesia & English

> **Prioritas:** 🟢 MEDIUM  
> **Estimasi:** 2-3 hari  
> **Kesulitan:** ⭐⭐ Mudah  
> **Prerequisite:** Tidak ada — bisa dikerjakan independen

---

## 📋 Deskripsi

Saat ini semua teks di frontend hardcoded dalam campuran Bahasa Indonesia dan English.  
Issue ini menambahkan sistem **internationalization (i18n)** sederhana agar storefront dan admin dashboard bisa switch bahasa.

---

## 🎯 Acceptance Criteria

- [ ] Buyer bisa switch bahasa (ID/EN) via toggle di navbar
- [ ] Admin dashboard juga support switch bahasa
- [ ] Semua teks UI diambil dari file terjemahan, bukan hardcoded
- [ ] Pilihan bahasa disimpan di `localStorage`
- [ ] Minimal 2 bahasa: Bahasa Indonesia dan English

---

## 📐 Langkah-Langkah

### Langkah 1: Buat File Terjemahan

**File:** `crates/web/static/i18n/id.json`

```json
{
    "nav": {
        "home": "Beranda",
        "products": "Produk",
        "cart": "Keranjang",
        "orders": "Pesanan",
        "profile": "Profil",
        "login": "Masuk",
        "register": "Daftar",
        "logout": "Keluar",
        "wishlist": "Favorit"
    },
    "product": {
        "add_to_cart": "Tambah ke Keranjang",
        "buy_now": "Beli Sekarang",
        "out_of_stock": "Stok Habis",
        "price": "Harga",
        "stock": "Stok tersedia",
        "description": "Deskripsi",
        "reviews": "Ulasan"
    },
    "cart": {
        "title": "Keranjang Belanja",
        "empty": "Keranjang kosong",
        "subtotal": "Subtotal",
        "checkout": "Checkout",
        "remove": "Hapus",
        "quantity": "Jumlah"
    },
    "checkout": {
        "title": "Checkout",
        "shipping_address": "Alamat Pengiriman",
        "payment_method": "Metode Pembayaran",
        "place_order": "Buat Pesanan",
        "promo_code": "Kode Promo",
        "apply": "Terapkan"
    },
    "order": {
        "history": "Riwayat Pesanan",
        "detail": "Detail Pesanan",
        "status": "Status",
        "total": "Total",
        "cancel": "Batalkan",
        "confirm_delivery": "Konfirmasi Diterima"
    },
    "auth": {
        "login_title": "Masuk ke Akun",
        "register_title": "Buat Akun Baru",
        "email": "Email",
        "password": "Kata Sandi",
        "forgot_password": "Lupa kata sandi?",
        "or_login_with": "Atau masuk dengan",
        "google_login": "Masuk dengan Google"
    },
    "common": {
        "loading": "Memuat...",
        "error": "Terjadi kesalahan",
        "success": "Berhasil",
        "save": "Simpan",
        "cancel": "Batal",
        "delete": "Hapus",
        "edit": "Edit",
        "search": "Cari...",
        "no_data": "Tidak ada data"
    }
}
```

**File:** `crates/web/static/i18n/en.json`

```json
{
    "nav": {
        "home": "Home",
        "products": "Products",
        "cart": "Cart",
        "orders": "Orders",
        "profile": "Profile",
        "login": "Login",
        "register": "Register",
        "logout": "Logout",
        "wishlist": "Wishlist"
    },
    "product": {
        "add_to_cart": "Add to Cart",
        "buy_now": "Buy Now",
        "out_of_stock": "Out of Stock",
        "price": "Price",
        "stock": "In Stock",
        "description": "Description",
        "reviews": "Reviews"
    },
    "cart": {
        "title": "Shopping Cart",
        "empty": "Your cart is empty",
        "subtotal": "Subtotal",
        "checkout": "Checkout",
        "remove": "Remove",
        "quantity": "Quantity"
    },
    "checkout": {
        "title": "Checkout",
        "shipping_address": "Shipping Address",
        "payment_method": "Payment Method",
        "place_order": "Place Order",
        "promo_code": "Promo Code",
        "apply": "Apply"
    },
    "order": {
        "history": "Order History",
        "detail": "Order Detail",
        "status": "Status",
        "total": "Total",
        "cancel": "Cancel",
        "confirm_delivery": "Confirm Delivery"
    },
    "auth": {
        "login_title": "Sign In",
        "register_title": "Create Account",
        "email": "Email",
        "password": "Password",
        "forgot_password": "Forgot password?",
        "or_login_with": "Or sign in with",
        "google_login": "Sign in with Google"
    },
    "common": {
        "loading": "Loading...",
        "error": "An error occurred",
        "success": "Success",
        "save": "Save",
        "cancel": "Cancel",
        "delete": "Delete",
        "edit": "Edit",
        "search": "Search...",
        "no_data": "No data available"
    }
}
```

### Langkah 2: Buat i18n Helper Module

**File:** `crates/web/static/store/store-i18n.js` (file baru)

```javascript
// i18n — Simple internationalization module
window.I18n = {
    currentLang: localStorage.getItem('lang') || 'id',
    translations: {},
    
    async init() {
        await this.loadLanguage(this.currentLang);
    },
    
    async loadLanguage(lang) {
        try {
            const res = await fetch(`/assets/i18n/${lang}.json`);
            this.translations = await res.json();
            this.currentLang = lang;
            localStorage.setItem('lang', lang);
            this.updatePage();
        } catch (e) {
            console.error('Failed to load language:', lang, e);
        }
    },
    
    // Get translation by dot-path: t('nav.home') → "Beranda"
    t(key) {
        const parts = key.split('.');
        let value = this.translations;
        for (const part of parts) {
            value = value?.[part];
        }
        return value || key;
    },
    
    // Switch language
    async switchTo(lang) {
        await this.loadLanguage(lang);
    },
    
    // Update all elements with data-i18n attribute
    updatePage() {
        document.querySelectorAll('[data-i18n]').forEach(el => {
            const key = el.getAttribute('data-i18n');
            el.textContent = this.t(key);
        });
        document.querySelectorAll('[data-i18n-placeholder]').forEach(el => {
            const key = el.getAttribute('data-i18n-placeholder');
            el.placeholder = this.t(key);
        });
    }
};

// Initialize on load
document.addEventListener('DOMContentLoaded', () => I18n.init());
```

### Langkah 3: Tambah Language Toggle di HTML

**File:** `crates/web/static/store.html`

Di navbar, tambahkan toggle button:

```html
<div class="lang-toggle">
    <button onclick="I18n.switchTo('id')" class="lang-btn" id="langId">🇮🇩 ID</button>
    <button onclick="I18n.switchTo('en')" class="lang-btn" id="langEn">🇬🇧 EN</button>
</div>
```

Tambahkan script tag:
```html
<script src="/assets/store/store-i18n.js" defer></script>
```

### Langkah 4: Update HTML Elements dengan `data-i18n`

Ganti teks hardcoded di `store.html` dengan atribut `data-i18n`:

**Sebelum:**
```html
<span>Keranjang</span>
```

**Sesudah:**
```html
<span data-i18n="nav.cart">Keranjang</span>
```

Ulangi untuk semua teks UI yang user-facing.

### Langkah 5: Update JavaScript untuk Pakai `I18n.t()`

**File:** Semua file di `crates/web/static/store/`

Ganti string hardcoded di JavaScript:

**Sebelum:**
```javascript
showStoreToast('Berhasil ditambahkan ke keranjang', 'success');
```

**Sesudah:**
```javascript
showStoreToast(I18n.t('common.success'), 'success');
```

### Langkah 6: Tambah CSS untuk Language Toggle

**File:** `crates/web/static/store.css`

```css
.lang-toggle {
    display: flex;
    gap: 4px;
    align-items: center;
}
.lang-btn {
    background: transparent;
    border: 1px solid rgba(255,255,255,0.3);
    color: inherit;
    padding: 4px 8px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.8rem;
    transition: all 0.2s;
}
.lang-btn:hover, .lang-btn.active {
    background: rgba(255,255,255,0.2);
    border-color: rgba(255,255,255,0.6);
}
```

### Langkah 7: (Opsional) i18n untuk Admin Dashboard

Ulangi langkah 2-6 untuk `index.html` dan file-file di `admin/`.

### Langkah 8: Verifikasi Manual

1. Buka storefront di browser
2. Klik toggle "🇬🇧 EN" → semua teks harus berubah ke English
3. Refresh halaman → bahasa harus tetap (tersimpan di localStorage)
4. Klik "🇮🇩 ID" → kembali ke Bahasa Indonesia

### Langkah 9: Verifikasi

```bash
cargo check --workspace
cargo test --workspace
```

> **Note:** Fitur i18n ini 100% frontend — tidak ada perubahan Rust backend.  
> Tidak perlu unit test backend baru, tapi pastikan file JSON valid.

---

## ⚠️ Perhatian

- **File JSON harus valid** — gunakan JSON validator sebelum commit
- Jangan ubah backend API response — terjemahan hanya di frontend
- Default language = `id` (Bahasa Indonesia) untuk UX lokal
- `data-i18n` attribute update otomatis saat switch bahasa
- Untuk teks dinamis dari JavaScript, gunakan `I18n.t('key')` function
- Pastikan semua file i18n JSON punya **key yang sama** (ID dan EN selaras)
