<script lang="ts">
  import { cart } from './cart.svelte';
  import { auth } from './auth.svelte';
  import { toast } from './toast.svelte';
  import { apiFetch } from './api';
  import { formatRupiah } from './currency';
  import { loadMidtransSnap } from './midtrans';
  import type { CreateOrderResponse, CouponValidationResult, Coupon } from './types';

  interface Props {
    isOpen: boolean;
    onClose: () => void;
  }

  let { isOpen, onClose }: Props = $props();
  let isSubmitting = $state(false);
  let notes = $state('');

  // Coupon / Promo state
  let promoInput = $state('');
  let isValidatingCoupon = $state(false);
  let appliedCoupon = $state<Coupon | null>(null);
  let discountAmount = $state(0);
  let promoError = $state<string | null>(null);

  const effectiveTotal = $derived(Math.max(0, cart.totalAmountCents - discountAmount));

  async function handleApplyPromo() {
    if (!promoInput.trim()) return;
    isValidatingCoupon = true;
    promoError = null;
    try {
      const res = await apiFetch<CouponValidationResult>('/api/v1/coupons/validate', {
        method: 'POST',
        body: JSON.stringify({
          code: promoInput.trim().toUpperCase(),
          order_amount: cart.totalAmountCents,
        }),
      });

      if (res.is_valid) {
        appliedCoupon = res.coupon || null;
        discountAmount = res.discount_amount;
        toast.success(`Kupon ${res.coupon?.code || promoInput.toUpperCase()} berhasil diterapkan!`);
      } else {
        promoError = res.message || 'Kupon tidak valid atau telah kedaluwarsa.';
        appliedCoupon = null;
        discountAmount = 0;
        toast.error(promoError);
      }
    } catch (err: any) {
      promoError = err.message || 'Gagal memvalidasi kupon.';
      appliedCoupon = null;
      discountAmount = 0;
      toast.error(promoError);
    } finally {
      isValidatingCoupon = false;
    }
  }

  function handleRemovePromo() {
    appliedCoupon = null;
    discountAmount = 0;
    promoInput = '';
    promoError = null;
    toast.info('Kupon dibatalkan.');
  }

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

      // 2. Buat pesanan ke Axum backend (tambahkan info kupon jika ada)
      let checkoutNotes = notes.trim();
      if (appliedCoupon) {
        checkoutNotes = checkoutNotes
          ? `${checkoutNotes} [Kupon: ${appliedCoupon.code} (-${formatRupiah(discountAmount)})]`
          : `[Kupon: ${appliedCoupon.code} (-${formatRupiah(discountAmount)})]`;
      }

      const payload = {
        items: cart.items.map(i => ({
          product_id: i.product.id,
          quantity: i.quantity,
        })),
        notes: checkoutNotes,
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
            handleRemovePromo();
            onClose();
          },
          onPending: () => {
            toast.info('Menunggu penyelesaian pembayaran.');
            cart.clear();
            handleRemovePromo();
            onClose();
          },
          onError: () => {
            toast.error('Pembayaran gagal atau dibatalkan.');
          },
          onClose: () => {
            toast.info('Jendela pembayaran ditutup.');
            cart.clear();
            handleRemovePromo();
            onClose();
          },
        });
      } else {
        toast.success(`Pesanan #${res.order_id.slice(0, 8)} berhasil dibuat!`);
        cart.clear();
        handleRemovePromo();
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
        <div class="summary-row">
          <span>Subtotal:</span>
          <span>{formatRupiah(cart.totalAmountCents)}</span>
        </div>
        {#if appliedCoupon}
          <div class="summary-row discount-row">
            <span>🏷️ Diskon ({appliedCoupon.code}):</span>
            <strong class="discount-val">-{formatRupiah(discountAmount)}</strong>
          </div>
        {/if}
        <div class="summary-row total-highlight">
          <span>Total Tagihan:</span>
          <strong class="price">{formatRupiah(effectiveTotal)}</strong>
        </div>
      </div>

      <!-- Promo Code Box -->
      <div class="promo-box">
        {#if appliedCoupon}
          <div class="applied-badge">
            <span class="badge-text">🏷️ Kupon <strong>{appliedCoupon.code}</strong> aktif (-{formatRupiah(discountAmount)})</span>
            <button type="button" class="btn-remove-coupon" onclick={handleRemovePromo}>Batal</button>
          </div>
        {:else}
          <div class="promo-input-group">
            <input
              type="text"
              placeholder="Punya Kode Promo? (cth: HEMAT20)"
              bind:value={promoInput}
              onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), handleApplyPromo())}
            />
            <button
              type="button"
              class="btn-apply-promo"
              onclick={handleApplyPromo}
              disabled={isValidatingCoupon || !promoInput.trim()}
            >
              {isValidatingCoupon ? 'Cek...' : 'Terapkan'}
            </button>
          </div>
          {#if promoError}
            <p class="promo-error">{promoError}</p>
          {/if}
        {/if}
      </div>

      <form onsubmit={handleCheckout} class="checkout-form">
        <label>
          Catatan Tambahan (Opsional):
          <textarea bind:value={notes} rows="2" placeholder="Contoh: Packing kayu / warna hitam"></textarea>
        </label>

        <div class="modal-actions">
          <button type="button" class="btn-secondary" onclick={onClose} disabled={isSubmitting}>Batal</button>
          <button type="submit" class="btn-primary" disabled={isSubmitting || cart.items.length === 0}>
            {isSubmitting ? 'Menyiapkan Transaksi...' : `Bayar Sekarang (${formatRupiah(effectiveTotal)}) 🚀`}
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
    margin-bottom: 1rem; display: flex; flex-direction: column; gap: 0.5rem;
  }
  .summary-row { display: flex; justify-content: space-between; font-size: 0.95rem; }
  .discount-row { color: #10b981; }
  .discount-val { color: #10b981; font-weight: bold; }
  .total-highlight { border-top: 1px solid #334155; padding-top: 0.5rem; font-size: 1.1rem; }
  .price { color: #38bdf8; }

  /* Promo Box */
  .promo-box { margin-bottom: 1.25rem; }
  .promo-input-group { display: flex; gap: 0.5rem; }
  .promo-input-group input {
    flex: 1; background: #1e293b; border: 1px solid #334155; border-radius: 6px;
    padding: 0.55rem 0.75rem; color: #fff; font-size: 0.85rem; text-transform: uppercase;
  }
  .promo-input-group input:focus { border-color: #0284c7; outline: none; }
  .btn-apply-promo {
    background: #0284c7; color: #fff; border: none; padding: 0.55rem 1rem;
    border-radius: 6px; font-weight: bold; cursor: pointer; font-size: 0.85rem;
    white-space: nowrap;
  }
  .btn-apply-promo:hover:not(:disabled) { background: #0369a1; }
  .btn-apply-promo:disabled { background: #475569; cursor: not-allowed; }
  .applied-badge {
    background: #064e3b; border: 1px solid #059669; padding: 0.6rem 0.8rem;
    border-radius: 6px; display: flex; justify-content: space-between; align-items: center;
  }
  .badge-text { font-size: 0.85rem; color: #a7f3d0; }
  .btn-remove-coupon {
    background: #7f1d1d; color: #fecaca; border: none; padding: 0.25rem 0.6rem;
    border-radius: 4px; font-size: 0.75rem; cursor: pointer;
  }
  .btn-remove-coupon:hover { background: #991b1b; }
  .promo-error { font-size: 0.8rem; color: #f87171; margin-top: 0.35rem; }

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

