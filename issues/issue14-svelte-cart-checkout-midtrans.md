# Issue 14: Shopping Cart, Checkout Modal & Integrasi Midtrans Snap JS

> **Prioritas:** 🔴 CRITICAL  
> **Estimasi:** 4-5 hari  
> **Kesulitan:** ⭐⭐⭐⭐ Menengah-Lanjut  
> **Prerequisite:** Issue 12 & Issue 13

---

## 📋 Deskripsi

Di sistem lama (`store-cart.js` dan `store-checkout.js`), keranjang belanja disimpan di `localStorage` dan checkout menggunakan script manual.  
Di Issue ini, kita akan:
1. Membangun **Cart Reactive State (Svelte 5)** dengan persistensi lokal (`localStorage`).
2. Membuat **Cart Drawer** (panel geser samping kanan) yang interaktif untuk mengatur kuantitas barang belanjaan.
3. Membuat **Checkout Modal** untuk memilih alamat pengiriman dan meninjau rincian biaya.
4. Mengintegrasikan **Midtrans Snap JavaScript SDK** secara dinamis di Svelte untuk menampilkan popup pembayaran yang mulus tanpa meninggalkan halaman.

---

## 🎯 Acceptance Criteria

- [ ] Keranjang belanja tersimpan otomatis di `localStorage` (`program1_cart`).
- [ ] Tombol `+` dan `-` di cart drawer dapat menambah atau mengurangi kuantitas item, serta menghapus item jika kuantitas mencapai 0.
- [ ] Cart badge di Navbar memperbarui total item secara reaktif.
- [ ] Checkout Modal meminta autentikasi (memicu `AuthModal` jika user belum login).
- [ ] Mengirim permintaan pembuatan pesanan ke backend Rust via `POST /v1/orders`.
- [ ] Popup Midtrans Snap berhasil dimuat dan terbuka saat `snap_token` diterima dari backend.
- [ ] Keranjang belanja otomatis dikosongkan saat pembayaran berhasil (`onSuccess`).

---

## 📐 Langkah-Langkah Pengerjaan

### Langkah 1: Definisikan Tipe Data Cart & Order

**File:** `frontend/src/lib/types/cart.ts`

```typescript
import type { Product } from './catalog';

export interface CartItem {
  product: Product;
  quantity: number;
}

export interface CreateOrderItemPayload {
  product_id: string;
  quantity: number;
}

export interface CreateOrderPayload {
  items: CreateOrderItemPayload[];
  shipping_address_id?: string;
  notes?: string;
}

export interface CreateOrderResponse {
  order_id: string;
  total_amount_cents: number;
  snap_token?: string;
  redirect_url?: string;
  status: string;
}
```

---

### Langkah 2: Buat Svelte Cart Store (Persisten)

**File:** `frontend/src/lib/stores/cart.svelte.ts`

```typescript
import type { Product } from '$lib/types/catalog';
import type { CartItem } from '$lib/types/cart';
import { toast } from './toast.svelte';

const STORAGE_KEY = 'program1_cart';

class CartState {
  items = $state<CartItem[]>([]);
  isOpen = $state(false);

  constructor() {
    if (typeof window !== 'undefined') {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved) {
        try {
          this.items = JSON.parse(saved);
        } catch {
          this.items = [];
        }
      }
    }
  }

  private save() {
    if (typeof window !== 'undefined') {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.items));
    }
  }

  // Reactive Derived Values
  get totalItems(): number {
    return this.items.reduce((sum, item) => sum + item.quantity, 0);
  }

  get totalAmountCents(): number {
    return this.items.reduce((sum, item) => sum + (item.product.price_cents * item.quantity), 0);
  }

  addItem(product: Product, quantity = 1) {
    if (product.stock <= 0) {
      toast.error('Maaf, produk ini sedang habis.');
      return;
    }

    const existingIndex = this.items.findIndex(i => i.product.id === product.id);
    if (existingIndex > -1) {
      const currentQty = this.items[existingIndex].quantity;
      if (currentQty + quantity > product.stock) {
        toast.error(`Maksimal stok tersedia hanya ${product.stock} unit.`);
        return;
      }
      this.items[existingIndex].quantity += quantity;
    } else {
      this.items = [...this.items, { product, quantity }];
    }

    this.save();
    toast.success(`${product.name} dimasukkan ke keranjang!`);
  }

  updateQuantity(productId: string, delta: number) {
    const item = this.items.find(i => i.product.id === productId);
    if (!item) return;

    const newQty = item.quantity + delta;
    if (newQty <= 0) {
      this.removeItem(productId);
    } else if (newQty > item.product.stock) {
      toast.error(`Stok maksimal ${item.product.stock} unit.`);
    } else {
      item.quantity = newQty;
      this.save();
    }
  }

  removeItem(productId: string) {
    this.items = this.items.filter(i => i.product.id !== productId);
    this.save();
  }

  clear() {
    this.items = [];
    this.save();
  }
}

export const cart = new CartState();
```

---

### Langkah 3: Integrasi Midtrans Snap JS SDK Loader

**File:** `frontend/src/lib/utils/midtrans.ts`

```typescript
declare global {
  interface Window {
    snap: {
      pay: (
        token: string,
        options: {
          onSuccess?: (result: any) => void;
          onPending?: (result: any) => void;
          onError?: (result: any) => void;
          onClose?: () => void;
        }
      ) => void;
    };
  }
}

export function loadMidtransSnap(clientKey: string, isProduction = false): Promise<void> {
  return new Promise((resolve, reject) => {
    if (window.snap) {
      return resolve();
    }

    const scriptUrl = isProduction
      ? 'https://app.midtrans.com/snap/snap.js'
      : 'https://app.sandbox.midtrans.com/snap/snap.js';

    const script = document.createElement('script');
    script.src = scriptUrl;
    script.setAttribute('data-client-key', clientKey);
    script.async = true;

    script.onload = () => resolve();
    script.onerror = () => reject(new Error('Gagal memuat script Midtrans Snap SDK'));

    document.body.appendChild(script);
  });
}
```

---

### Langkah 4: Buat Komponen `CartDrawer.svelte`

**File:** `frontend/src/lib/components/CartDrawer.svelte`

```svelte
<script lang="ts">
  import { cart } from '$lib/stores/cart.svelte';
  import { auth } from '$lib/stores/auth.svelte';
  import { formatRupiah } from '$lib/utils/currency';

  interface Props {
    onProceedCheckout: () => void;
  }

  let { onProceedCheckout }: Props = $props();

  function handleCheckout() {
    if (!auth.user) {
      auth.isModalOpen = true;
      cart.isOpen = false;
      return;
    }
    cart.isOpen = false;
    onProceedCheckout();
  }
</script>

{#if cart.isOpen}
  <div class="drawer-backdrop" onclick={() => cart.isOpen = false}>
    <div class="drawer-panel" onclick={(e) => e.stopPropagation()}>
      <div class="drawer-header">
        <h2>🛒 Keranjang Belanja ({cart.totalItems})</h2>
        <button onclick={() => cart.isOpen = false} class="btn-close">&times;</button>
      </div>

      <div class="drawer-body">
        {#if cart.items.length === 0}
          <div class="empty-cart">
            <p>Keranjang belanja Anda masih kosong.</p>
          </div>
        {:else}
          <div class="cart-items">
            {#each cart.items as item (item.product.id)}
              <div class="cart-item">
                <div class="item-info">
                  <h4>{item.product.name}</h4>
                  <p class="item-price">{formatRupiah(item.product.price_cents)}</p>
                </div>
                <div class="item-controls">
                  <button onclick={() => cart.updateQuantity(item.product.id, -1)} class="btn-qty">-</button>
                  <span class="qty">{item.quantity}</span>
                  <button onclick={() => cart.updateQuantity(item.product.id, 1)} class="btn-qty">+</button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      {#if cart.items.length > 0}
        <div class="drawer-footer">
          <div class="subtotal-row">
            <span>Subtotal:</span>
            <strong>{formatRupiah(cart.totalAmountCents)}</strong>
          </div>
          <button onclick={handleCheckout} class="btn-checkout">
            Lanjut ke Pembayaran
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .drawer-backdrop {
    position: fixed; inset: 0; background: rgba(0,0,0,0.5); z-index: 1000;
    display: flex; justify-content: flex-end;
  }
  .drawer-panel {
    background: #1e293b; width: 100%; max-width: 400px; height: 100%;
    display: flex; flex-direction: column; color: #f8fafc;
    box-shadow: -4px 0 16px rgba(0,0,0,0.4);
    animation: slideLeft 0.25s ease-out;
  }
  .drawer-header {
    padding: 1.25rem; border-bottom: 1px solid #334155;
    display: flex; justify-content: space-between; align-items: center;
  }
  .drawer-header h2 { margin: 0; font-size: 1.2rem; }
  .btn-close { background: none; border: none; font-size: 1.5rem; color: #fff; cursor: pointer; }
  .drawer-body { flex: 1; overflow-y: auto; padding: 1.25rem; }
  .empty-cart { text-align: center; color: #94a3b8; padding: 3rem 0; }
  .cart-items { display: flex; flex-direction: column; gap: 1rem; }
  .cart-item {
    display: flex; justify-content: space-between; align-items: center;
    background: #0f172a; padding: 0.75rem 1rem; border-radius: 8px;
    border: 1px solid #334155;
  }
  .item-info h4 { margin: 0 0 0.25rem; font-size: 0.95rem; }
  .item-price { margin: 0; color: #38bdf8; font-size: 0.9rem; font-weight: bold; }
  .item-controls { display: flex; align-items: center; gap: 0.5rem; }
  .btn-qty {
    background: #334155; color: #fff; border: none; width: 28px; height: 28px;
    border-radius: 4px; cursor: pointer; font-weight: bold;
  }
  .qty { min-width: 20px; text-align: center; font-size: 0.9rem; }
  .drawer-footer { padding: 1.25rem; border-top: 1px solid #334155; background: #0f172a; }
  .subtotal-row { display: flex; justify-content: space-between; margin-bottom: 1rem; font-size: 1.1rem; }
  .btn-checkout {
    width: 100%; background: #0284c7; color: #fff; border: none;
    padding: 0.8rem; border-radius: 6px; font-weight: bold; font-size: 1rem; cursor: pointer;
  }
  @keyframes slideLeft {
    from { transform: translateX(100%); }
    to { transform: translateX(0); }
  }
</style>
```

---

### Langkah 5: Buat Komponen `CheckoutModal.svelte`

**File:** `frontend/src/lib/components/CheckoutModal.svelte`

```svelte
<script lang="ts">
  import { cart } from '$lib/stores/cart.svelte';
  import { toast } from '$lib/stores/toast.svelte';
  import { apiFetch } from '$lib/api/client';
  import { formatRupiah } from '$lib/utils/currency';
  import { loadMidtransSnap } from '$lib/utils/midtrans';
  import type { CreateOrderResponse } from '$lib/types/cart';

  interface Props {
    isOpen: boolean;
    onClose: () => void;
  }

  let { isOpen, onClose }: Props = $props();
  let isSubmitting = $state(false);
  let notes = $state('');

  async function handlePay() {
    if (cart.items.length === 0) return;

    isSubmitting = true;
    try {
      // 1. Ambil config Midtrans Client Key dari backend
      const config = await apiFetch<{ client_key: string }>('/buyer/payment-config');
      await loadMidtransSnap(config.client_key, false);

      // 2. Buat pesanan ke backend Rust Axum
      const payload = {
        items: cart.items.map(i => ({
          product_id: i.product.id,
          quantity: i.quantity,
        })),
        notes,
      };

      const orderRes = await apiFetch<CreateOrderResponse>('/v1/orders', {
        method: 'POST',
        body: JSON.stringify(payload),
      });

      // 3. Jika backend mengembalikan Snap Token, buka popup Midtrans
      if (orderRes.snap_token) {
        window.snap.pay(orderRes.snap_token, {
          onSuccess: (result) => {
            toast.success('Pembayaran Berhasil! Pesanan sedang diproses.');
            cart.clear();
            onClose();
          },
          onPending: (result) => {
            toast.info('Menunggu penyelesaian pembayaran.');
            cart.clear();
            onClose();
          },
          onError: (err) => {
            toast.error('Pembayaran gagal atau dibatalkan.');
          },
          onClose: () => {
            toast.info('Jendela pembayaran ditutup.');
          },
        });
      } else {
        toast.success(`Pesanan #${orderRes.order_id} berhasil dibuat!`);
        cart.clear();
        onClose();
      }
    } catch (err: any) {
      toast.error(err.message || 'Gagal memproses pesanan.');
    } finally {
      isSubmitting = false;
    }
  }
</script>

{#if isOpen}
  <div class="modal-backdrop" onclick={onClose}>
    <div class="modal-card" onclick={(e) => e.stopPropagation()}>
      <h3>Konfirmasi & Pembayaran Pesanan</h3>

      <div class="summary-box">
        <p><strong>Jumlah Item:</strong> {cart.totalItems} barang</p>
        <p><strong>Total Pembayaran:</strong> <span class="total">{formatRupiah(cart.totalAmountCents)}</span></p>
      </div>

      <label>
        Catatan untuk Penjual (Opsional):
        <textarea bind:value={notes} rows="2" placeholder="Contoh: Tolong packing rapi"></textarea>
      </label>

      <div class="actions">
        <button onclick={onClose} class="btn-cancel" disabled={isSubmitting}>Batal</button>
        <button onclick={handlePay} class="btn-pay" disabled={isSubmitting}>
          {isSubmitting ? 'Menyiapkan Pembayaran...' : 'Bayar Sekarang 💳'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed; inset: 0; background: rgba(0,0,0,0.6);
    display: flex; align-items: center; justify-content: center; z-index: 1000;
  }
  .modal-card {
    background: #1e293b; border: 1px solid #334155; border-radius: 12px;
    width: 90%; max-width: 450px; padding: 1.5rem; color: #f8fafc;
  }
  .summary-box {
    background: #0f172a; padding: 1rem; border-radius: 8px;
    margin: 1rem 0; border: 1px solid #334155;
  }
  .summary-box p { margin: 0.35rem 0; font-size: 0.95rem; }
  .total { color: #38bdf8; font-weight: bold; font-size: 1.1rem; }
  textarea {
    width: 100%; box-sizing: border-box; background: #0f172a;
    border: 1px solid #334155; border-radius: 6px; padding: 0.6rem;
    color: #fff; margin-top: 0.4rem; font-family: inherit;
  }
  .actions { display: flex; justify-content: flex-end; gap: 0.75rem; margin-top: 1.25rem; }
  .btn-cancel { background: #475569; color: #fff; border: none; padding: 0.6rem 1.2rem; border-radius: 6px; cursor: pointer; }
  .btn-pay { background: #0284c7; color: #fff; border: none; padding: 0.6rem 1.2rem; border-radius: 6px; font-weight: bold; cursor: pointer; }
</style>
```

---

## 🧪 Pengujian & Verifikasi

1. Buka browser ke `http://localhost:5173`.
2. Masukkan beberapa produk ke keranjang belanja melalui tombol `+ Keranjang`.
3. Buka Cart Drawer, coba tambah dan kurangi item. Pastikan subtotal rupiah dihitung dengan tepat.
4. Klik tombol **Lanjut ke Pembayaran**.
5. Masuk/Login jika diminta.
6. Pada Modal Checkout, klik **Bayar Sekarang 💳**.
7. Pastikan modal popup Snap Sandbox Midtrans muncul dengan QRIS/Virtual Account dummy.
8. Selesaikan transaksi sandbox, verifikasi keranjang belanja otomatis kosong dan toast sukses muncul.
9. Jalankan tes Rust: `cargo test --workspace` untuk memastikan tidak ada error pada endpoint backend order.

---

## ⚠️ Hal Penting & Gotchas

- **Midtrans Client Key:** Jangan menaruh hardcoded Midtrans Server Key di frontend. Frontend HANYA membutuhkan Client Key dari endpoint backend `GET /buyer/payment-config`.
- **Concurrency & Stock:** Jika stok barang habis sebelum pembayaran selesai, backend Rust akan mengembalikan status `422 Insufficient Stock`. API client otomatis menangkap ini dan menampilkan toast error ke user.
