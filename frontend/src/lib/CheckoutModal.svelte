<script lang="ts">
  import { cart } from './cart.svelte';
  import { auth } from './auth.svelte';
  import { toast } from './toast.svelte';
  import { apiFetch } from './api';
  import { formatRupiah } from './currency';
  import { loadMidtransSnap } from './midtrans';
  import type { CreateOrderResponse } from './types';

  interface Props {
    isOpen: boolean;
    onClose: () => void;
  }

  let { isOpen, onClose }: Props = $props();
  let isSubmitting = $state(false);
  let notes = $state('');

  async function handleCheckout(e: Event) {
    e.preventDefault();
    if (cart.items.length === 0) return;

    if (!auth.user) {
      toast.error('Silakan login terlebih dahulu untuk melakukan checkout.');
      auth.isModalOpen = true;
      return;
    }

    isSubmitting = true;
    try {
      // 1. Ambil config Client Key Midtrans
      const config = await apiFetch<{ client_key: string }>('/buyer/payment-config').catch(() => ({ client_key: 'SB-Mid-client-dummy' }));
      await loadMidtransSnap(config.client_key, false).catch(() => {});

      // 2. Buat pesanan ke Axum backend
      const payload = {
        items: cart.items.map(i => ({
          product_id: i.product.id,
          quantity: i.quantity,
        })),
        notes,
      };

      const res = await apiFetch<CreateOrderResponse>('/v1/orders', {
        method: 'POST',
        body: JSON.stringify(payload),
      });

      // 3. Tangani Midtrans Snap popup jika snap_token dikembalikan
      if (res.snap_token && window.snap) {
        window.snap.pay(res.snap_token, {
          onSuccess: () => {
            toast.success('Pembayaran Berhasil! Pesanan sedang diproses.');
            cart.clear();
            onClose();
          },
          onPending: () => {
            toast.info('Menunggu penyelesaian pembayaran.');
            cart.clear();
            onClose();
          },
          onError: () => {
            toast.error('Pembayaran gagal atau dibatalkan.');
          },
          onClose: () => {
            toast.info('Jendela pembayaran ditutup.');
            cart.clear();
            onClose();
          },
        });
      } else {
        toast.success(`Pesanan #${res.order_id.slice(0, 8)} berhasil dibuat!`);
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
  <div class="modal-overlay" onclick={onClose} role="button" tabindex="0" onkeydown={(e) => e.key === 'Escape' && onClose()}>
    <div class="modal-card" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <div class="modal-header">
        <h3>💳 Konfirmasi Pembayaran</h3>
        <button class="close-btn" onclick={onClose}>&times;</button>
      </div>

      <div class="order-summary">
        <div class="summary-row">
          <span>Total Item:</span>
          <strong>{cart.totalItems} barang</strong>
        </div>
        <div class="summary-row total-highlight">
          <span>Total Tagihan:</span>
          <strong class="price">{formatRupiah(cart.totalAmountCents)}</strong>
        </div>
      </div>

      <form onsubmit={handleCheckout} class="checkout-form">
        <label>
          Catatan Tambahan (Opsional):
          <textarea bind:value={notes} rows="2" placeholder="Contoh: Packing kayu / warna hitam"></textarea>
        </label>

        <div class="modal-actions">
          <button type="button" class="btn-secondary" onclick={onClose} disabled={isSubmitting}>Batal</button>
          <button type="submit" class="btn-primary" disabled={isSubmitting || cart.items.length === 0}>
            {isSubmitting ? 'Menyiapkan Transaksi...' : 'Bayar Sekarang 🚀'}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.7);
    display: flex; align-items: center; justify-content: center; z-index: 1000;
  }
  .modal-card {
    background: #0f172a; border: 1px solid #334155; border-radius: 12px;
    width: 90%; max-width: 440px; padding: 1.5rem; color: #f8fafc;
    box-shadow: 0 10px 30px rgba(0,0,0,0.5);
  }
  .modal-header {
    display: flex; justify-content: space-between; align-items: center;
    border-bottom: 1px solid #334155; padding-bottom: 0.75rem; margin-bottom: 1rem;
  }
  .modal-header h3 { margin: 0; font-size: 1.2rem; }
  .close-btn { background: none; border: none; font-size: 1.4rem; color: #94a3b8; cursor: pointer; }
  .order-summary {
    background: #1e293b; padding: 1rem; border-radius: 8px; border: 1px solid #334155;
    margin-bottom: 1.25rem; display: flex; flex-direction: column; gap: 0.5rem;
  }
  .summary-row { display: flex; justify-content: space-between; font-size: 0.95rem; }
  .total-highlight { border-top: 1px solid #334155; padding-top: 0.5rem; font-size: 1.1rem; }
  .price { color: #38bdf8; }
  .checkout-form { display: flex; flex-direction: column; gap: 1rem; }
  label { font-size: 0.85rem; color: #cbd5e1; display: flex; flex-direction: column; gap: 0.35rem; }
  textarea {
    background: #1e293b; border: 1px solid #334155; border-radius: 6px;
    padding: 0.6rem; color: #fff; font-family: inherit; resize: vertical;
  }
  textarea:focus { border-color: #0284c7; outline: none; }
  .modal-actions { display: flex; justify-content: flex-end; gap: 0.75rem; margin-top: 0.5rem; }
  .btn-secondary {
    background: #334155; color: #fff; border: none; padding: 0.65rem 1.2rem;
    border-radius: 6px; cursor: pointer; font-weight: 500;
  }
  .btn-primary {
    background: #0284c7; color: #fff; border: none; padding: 0.65rem 1.2rem;
    border-radius: 6px; cursor: pointer; font-weight: bold;
  }
  .btn-primary:hover:not(:disabled) { background: #0369a1; }
  .btn-primary:disabled { background: #475569; cursor: not-allowed; }
</style>
