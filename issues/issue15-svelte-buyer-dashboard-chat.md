# Issue 15: Buyer Dashboard, Riwayat Pesanan & Real-time Live Chat Widget

> **Prioritas:** 🟡 HIGH  
> **Estimasi:** 3-4 hari  
> **Kesulitan:** ⭐⭐⭐ Menengah  
> **Prerequisite:** Issue 12 & Issue 14

---

## 📋 Deskripsi

Di sistem lama (`store-main.js` dan `store-chat.js`), riwayat pesanan pembeli dan floating chat widget dibuat menggunakan jQuery-style DOM injection yang rentan conflict dan sulit di-maintain.  
Di Issue ini, kita akan:
1. Membangun **Buyer Dashboard (Svelte 5)** untuk melihat riwayat pesanan, detail transaksi, dan konfirmasi penerimaan barang.
2. Membangun modul manajemen alamat pengiriman pembeli.
3. Membangun **Floating Live Chat Widget** yang selalu aktif di pojok kanan bawah, terintegrasi dengan backend Rust Axum (`/v1/chat/messages`) menggunakan polling pintar dan auto-scroll.

---

## 🎯 Acceptance Criteria

- [ ] Tampilan **Riwayat Pesanan** (`BuyerOrders.svelte`) menampilkan daftar transaksi dengan badge status berwarna (`PENDING_PAYMENT`, `PAID`, `PROCESSING`, `SHIPPED`, `DELIVERED`, `CANCELLED`).
- [ ] Pembeli dapat menekan tombol **"Konfirmasi Selesai"** pada pesanan berstatus `SHIPPED` untuk memicu status `DELIVERED`.
- [ ] Widget **Live Chat** (`LiveChatWidget.svelte`) melayang di pojok kanan bawah layar.
- [ ] Chat widget memiliki badge counter pesan yang belum dibaca.
- [ ] Pembeli dapat mengirim pesan teks dan melihat balasan dari Admin/Seller dengan auto-scroll ke pesan terbaru.
- [ ] Polling chat berjalan secara efisien (interval 4-5 detik) dan otomatis pause ketika tab browser tidak aktif (`document.hidden`).

---

## 📐 Langkah-Langkah Pengerjaan

### Langkah 1: Definisikan Tipe Data Order & Chat

**File:** `frontend/src/lib/types/buyer.ts`

```typescript
export interface BuyerOrderItem {
  product_id: string;
  product_name: string;
  quantity: number;
  price_cents: number;
}

export interface BuyerOrder {
  id: string;
  total_amount_cents: number;
  status: 'PENDING_PAYMENT' | 'PAID' | 'PROCESSING' | 'SHIPPED' | 'DELIVERED' | 'CANCELLED';
  tracking_number?: string | null;
  items: BuyerOrderItem[];
  created_at: string;
}

export interface ChatMessage {
  id: string;
  sender_role: 'Buyer' | 'Seller' | 'Admin';
  sender_name: string;
  content: string;
  created_at: string;
}
```

---

### Langkah 2: Buat Komponen Riwayat Pesanan `BuyerOrders.svelte`

**File:** `frontend/src/lib/components/BuyerOrders.svelte`

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { apiFetch } from '$lib/api/client';
  import { toast } from '$lib/stores/toast.svelte';
  import { formatRupiah } from '$lib/utils/currency';
  import type { BuyerOrder } from '$lib/types/buyer';

  let orders = $state<BuyerOrder[]>([]);
  let loading = $state(true);

  async function loadOrders() {
    loading = true;
    try {
      orders = await apiFetch<BuyerOrder[]>('/buyer/orders');
    } catch (err: any) {
      toast.error(err.message || 'Gagal mengambil riwayat pesanan');
    } finally {
      loading = false;
    }
  }

  async function confirmDelivery(orderId: string) {
    if (!confirm('Apakah Anda yakin pesanan telah diterima dengan baik?')) return;
    try {
      await apiFetch(`/buyer/orders/${orderId}/confirm-delivery`, { method: 'POST' });
      toast.success('Pesanan selesai! Terima kasih.');
      await loadOrders();
    } catch (err: any) {
      toast.error(err.message || 'Gagal mengonfirmasi penerimaan');
    }
  }

  onMount(() => {
    loadOrders();
  });
</script>

<div class="orders-container">
  <h2>📦 Riwayat Pesanan Saya</h2>

  {#if loading}
    <p class="loading">Memuat daftar pesanan...</p>
  {:else if orders.length === 0}
    <div class="empty">
      <p>Belum ada pesanan. Yuk mulai berbelanja!</p>
    </div>
  {:else}
    <div class="orders-list">
      {#each orders as order (order.id)}
        <div class="order-card">
          <div class="order-head">
            <div>
              <span class="order-id">#{order.id.slice(0, 8)}</span>
              <small class="order-date">{new Date(order.created_at).toLocaleDateString('id-ID')}</small>
            </div>
            <span class="status-badge {order.status.toLowerCase()}">{order.status}</span>
          </div>

          <div class="order-items">
            {#each order.items as item}
              <div class="item-row">
                <span>{item.product_name} &times; {item.quantity}</span>
                <span>{formatRupiah(item.price_cents * item.quantity)}</span>
              </div>
            {/each}
          </div>

          <div class="order-foot">
            <div class="total">
              <span>Total Pesanan:</span>
              <strong>{formatRupiah(order.total_amount_cents)}</strong>
            </div>

            {#if order.status === 'SHIPPED'}
              <button onclick={() => confirmDelivery(order.id)} class="btn-confirm">
                Konfirmasi Terima
              </button>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .orders-container { max-width: 800px; margin: 2rem auto; padding: 0 1rem; color: #f8fafc; }
  .orders-list { display: flex; flex-direction: column; gap: 1rem; }
  .order-card {
    background: #1e293b; border: 1px solid #334155; border-radius: 10px; padding: 1.25rem;
  }
  .order-head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.75rem; }
  .order-id { font-weight: bold; color: #38bdf8; margin-right: 0.5rem; }
  .order-date { color: #94a3b8; }
  .status-badge {
    padding: 0.25rem 0.6rem; border-radius: 4px; font-size: 0.75rem; font-weight: bold;
  }
  .status-badge.pending_payment { background: #d97706; color: #fff; }
  .status-badge.paid, .status-badge.processing { background: #0284c7; color: #fff; }
  .status-badge.shipped { background: #7c3aed; color: #fff; }
  .status-badge.delivered { background: #059669; color: #fff; }
  .status-badge.cancelled { background: #dc2626; color: #fff; }
  .order-items { border-top: 1px solid #334155; border-bottom: 1px solid #334155; padding: 0.75rem 0; margin: 0.75rem 0; }
  .item-row { display: flex; justify-content: space-between; font-size: 0.9rem; margin-bottom: 0.25rem; }
  .order-foot { display: flex; justify-content: space-between; align-items: center; }
  .total strong { color: #38bdf8; margin-left: 0.5rem; font-size: 1.1rem; }
  .btn-confirm {
    background: #059669; color: white; border: none; padding: 0.5rem 1rem;
    border-radius: 6px; font-weight: bold; cursor: pointer;
  }
  .loading, .empty { text-align: center; color: #94a3b8; padding: 3rem; }
</style>
```

---

### Langkah 3: Buat Komponen `LiveChatWidget.svelte`

**File:** `frontend/src/lib/components/LiveChatWidget.svelte`

```svelte
<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { apiFetch } from '$lib/api/client';
  import { auth } from '$lib/stores/auth.svelte';
  import type { ChatMessage } from '$lib/types/buyer';

  let isOpen = $state(false);
  let messages = $state<ChatMessage[]>([]);
  let inputText = $state('');
  let isSending = $state(false);
  let chatBoxEl: HTMLDivElement;
  let pollInterval: ReturnType<typeof setInterval>;

  async function fetchMessages() {
    if (!auth.user || document.hidden) return;
    try {
      const data = await apiFetch<ChatMessage[]>('/v1/chat/messages');
      const hasNew = data.length > messages.length;
      messages = data;
      if (hasNew && isOpen) {
        await tick();
        scrollToBottom();
      }
    } catch {
      // Abaikan error polling background
    }
  }

  function scrollToBottom() {
    if (chatBoxEl) {
      chatBoxEl.scrollTop = chatBoxEl.scrollHeight;
    }
  }

  async function sendMessage(e: Event) {
    e.preventDefault();
    if (!inputText.trim() || isSending) return;

    const content = inputText.trim();
    inputText = '';
    isSending = true;

    try {
      await apiFetch('/v1/chat/messages', {
        method: 'POST',
        body: JSON.stringify({ content }),
      });
      await fetchMessages();
    } finally {
      isSending = false;
    }
  }

  function toggleChat() {
    isOpen = !isOpen;
    if (isOpen) {
      fetchMessages();
      setTimeout(scrollToBottom, 100);
    }
  }

  onMount(() => {
    pollInterval = setInterval(fetchMessages, 4000);
  });

  onDestroy(() => {
    clearInterval(pollInterval);
  });
</script>

<div class="chat-widget">
  {#if isOpen}
    <div class="chat-window">
      <div class="chat-header">
        <h4>💬 Chat Layanan Pelanggan</h4>
        <button onclick={toggleChat} class="btn-close">&times;</button>
      </div>

      <div class="chat-messages" bind:this={chatBoxEl}>
        {#if !auth.user}
          <p class="login-prompt">Silakan login untuk mengirim pesan kepada admin.</p>
        {:else if messages.length === 0}
          <p class="empty-chat">Belum ada percakapan. Mulai percakapan sekarang!</p>
        {:else}
          {#each messages as msg (msg.id)}
            <div class="msg-bubble {msg.sender_role.toLowerCase()}">
              <span class="sender">{msg.sender_name}</span>
              <p class="text">{msg.content}</p>
            </div>
          {/each}
        {/if}
      </div>

      {#if auth.user}
        <form onsubmit={sendMessage} class="chat-form">
          <input
            type="text"
            placeholder="Tulis pesan..."
            bind:value={inputText}
            disabled={isSending}
          />
          <button type="submit" disabled={isSending || !inputText.trim()}>Kirim</button>
        </form>
      {/if}
    </div>
  {/if}

  <button class="chat-bubble-btn" onclick={toggleChat} aria-label="Buka Live Chat">
    💬
  </button>
</div>

<style>
  .chat-widget { position: fixed; bottom: 1.5rem; right: 1.5rem; z-index: 1000; }
  .chat-bubble-btn {
    width: 56px; height: 56px; border-radius: 50%; background: #0284c7;
    color: white; font-size: 1.5rem; border: none; cursor: pointer;
    box-shadow: 0 4px 16px rgba(0,0,0,0.4); transition: transform 0.2s;
  }
  .chat-bubble-btn:hover { transform: scale(1.08); background: #0369a1; }
  .chat-window {
    position: absolute; bottom: 70px; right: 0; width: 340px; height: 450px;
    background: #1e293b; border: 1px solid #334155; border-radius: 12px;
    display: flex; flex-direction: column; overflow: hidden;
    box-shadow: 0 8px 24px rgba(0,0,0,0.5);
  }
  .chat-header {
    background: #0f172a; padding: 0.8rem 1rem; border-bottom: 1px solid #334155;
    display: flex; justify-content: space-between; align-items: center; color: #f8fafc;
  }
  .chat-header h4 { margin: 0; font-size: 0.95rem; }
  .btn-close { background: none; border: none; color: #fff; font-size: 1.25rem; cursor: pointer; }
  .chat-messages { flex: 1; padding: 1rem; overflow-y: auto; display: flex; flex-direction: column; gap: 0.6rem; }
  .msg-bubble { max-width: 80%; padding: 0.6rem 0.8rem; border-radius: 8px; font-size: 0.85rem; }
  .msg-bubble.buyer { align-self: flex-end; background: #0284c7; color: #fff; }
  .msg-bubble.seller, .msg-bubble.admin { align-self: flex-start; background: #334155; color: #f8fafc; }
  .msg-bubble .sender { font-size: 0.7rem; opacity: 0.8; display: block; margin-bottom: 0.2rem; }
  .msg-bubble .text { margin: 0; word-break: break-word; }
  .chat-form { display: flex; border-top: 1px solid #334155; padding: 0.5rem; background: #0f172a; }
  .chat-form input { flex: 1; background: #1e293b; border: 1px solid #334155; border-radius: 4px; padding: 0.5rem; color: #fff; }
  .chat-form button { background: #0284c7; color: white; border: none; padding: 0.5rem 0.8rem; border-radius: 4px; margin-left: 0.5rem; cursor: pointer; }
  .login-prompt, .empty-chat { text-align: center; color: #94a3b8; font-size: 0.85rem; margin-top: 3rem; }
</style>
```

---

## 🧪 Pengujian & Verifikasi

1. Buka browser: `http://localhost:5173`.
2. Login sebagai buyer dan buat pesanan melalui alur checkout.
3. Buka tab / halaman **Riwayat Pesanan Saya**, pastikan order baru muncul dengan status yang sesuai.
4. Klik tombol mengambang `💬` di pojok kanan bawah, pastikan window live chat terbuka.
5. Ketik pesan dan kirim. Pastikan pesan muncul di daftar percakapan.
6. Buka tab browser lain (atau curl endpoint) untuk mengirim balasan admin, pastikan polling otomatis memperbarui isi pesan dalam 4 detik.
7. Jalankan unit test Rust: `cargo test --workspace`.

---

## ⚠️ Hal Penting & Gotchas

- **Efisiensi Polling:** Polling harus berhenti jika window/tab browser berada di background (`document.hidden`) untuk menghemat bandwidth dan beban CPU server.
- **Auto-scroll:** Gunakan `tick()` Svelte sebelum menghitung `scrollTop` agar DOM sudah selesai di-render sebelum scroll digerakkan.
