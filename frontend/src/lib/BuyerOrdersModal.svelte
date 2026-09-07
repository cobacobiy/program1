<script lang="ts">
  import { onMount } from 'svelte';
  import { apiFetch } from './api';
  import { toast } from './toast.svelte';
  import { formatRupiah } from './currency';
  import type { BuyerOrder, ProductReview, CreateReviewPayload } from './types';

  interface Props {
    isOpen: boolean;
    onClose: () => void;
  }

  let { isOpen, onClose }: Props = $props();
  let orders = $state<BuyerOrder[]>([]);
  let buyerReviews = $state<ProductReview[]>([]);
  let loading = $state(true);

  // Review Form Modal State
  let isReviewModalOpen = $state(false);
  let reviewTargetOrderId = $state('');
  let reviewTargetProductId = $state('');
  let reviewTargetProductName = $state('');
  let reviewRating = $state(5);
  let reviewText = $state('');
  let isSubmittingReview = $state(false);

  async function loadOrders() {
    loading = true;
    try {
      const [ord, rev] = await Promise.all([
        apiFetch<BuyerOrder[]>('/api/v1/buyer/orders').catch(() => apiFetch<BuyerOrder[]>('/buyer/orders')),
        apiFetch<ProductReview[]>('/api/v1/buyer/reviews').catch(() => []),
      ]);
      orders = ord;
      buyerReviews = rev || [];
    } catch (err: any) {
      toast.error(err.message || 'Gagal memuat riwayat pesanan.');
    } finally {
      loading = false;
    }
  }

  function hasReviewed(orderId: string, productId: string): boolean {
    return buyerReviews.some(
      (r) => r.order_id === orderId && r.product_id === productId,
    );
  }

  function openReviewModal(orderId: string, productId: string, productName: string) {
    reviewTargetOrderId = orderId;
    reviewTargetProductId = productId;
    reviewTargetProductName = productName;
    reviewRating = 5;
    reviewText = '';
    isReviewModalOpen = true;
  }

  async function submitReview() {
    if (reviewRating < 1 || reviewRating > 5) {
      toast.error('Pilih rating 1-5 bintang');
      return;
    }
    isSubmittingReview = true;
    try {
      const payload: CreateReviewPayload = {
        order_id: reviewTargetOrderId,
        product_id: reviewTargetProductId,
        rating: reviewRating,
        review_text: reviewText.trim() ? reviewText.trim() : undefined,
      };
      await apiFetch<ProductReview>('/api/v1/buyer/reviews', {
        method: 'POST',
        body: JSON.stringify(payload),
      });
      toast.success('Ulasan Anda berhasil dikirim! Terima kasih.');
      isReviewModalOpen = false;
      const rev = await apiFetch<ProductReview[]>('/api/v1/buyer/reviews').catch(() => []);
      buyerReviews = rev || [];
    } catch (err: any) {
      toast.error(err.message || 'Gagal mengirim ulasan.');
    } finally {
      isSubmittingReview = false;
    }
  }

  async function confirmDelivery(orderId: string) {
    if (!confirm('Apakah pesanan ini sudah sampai dan diterima dengan baik?')) return;
    try {
      await apiFetch(`/buyer/orders/${orderId}/confirm-delivery`, { method: 'POST' });
      toast.success('Pesanan telah diselesaikan! Terima kasih.');
      await loadOrders();
    } catch (err: any) {
      toast.error(err.message || 'Gagal mengonfirmasi penerimaan.');
    }
  }

  onMount(() => {
    if (isOpen) loadOrders();
  });

  $effect(() => {
    if (isOpen) {
      loadOrders();
    }
  });
</script>

{#if isOpen}
  <div class="modal-overlay" onclick={onClose} role="button" tabindex="0" onkeydown={(e) => e.key === 'Escape' && onClose()}>
    <div class="modal-card" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="dialog" aria-modal="true" tabindex="-1">
      <div class="modal-header">
        <h3>📦 Riwayat Pesanan Saya</h3>
        <button class="close-btn" onclick={onClose}>&times;</button>
      </div>

      <div class="orders-body">
        {#if loading}
          <p class="status-msg">Memuat daftar pesanan...</p>
        {:else if orders.length === 0}
          <div class="empty-orders">
            <p>Anda belum memiliki riwayat pesanan.</p>
          </div>
        {:else}
          <div class="orders-list">
            {#each orders as o (o.id)}
              <div class="order-box">
                <div class="box-top">
                  <div>
                    <span class="oid">#{o.id.slice(0, 8)}</span>
                    <small class="date">{new Date(o.created_at).toLocaleDateString('id-ID')}</small>
                  </div>
                  <span class="status-pill {o.status.toLowerCase()}">{o.status}</span>
                </div>

                <div class="items-summary">
                  {#each o.items as item}
                    <div class="item-line">
                      <div class="item-info-col">
                        <span class="item-name">{item.product_name} &times; {item.quantity}</span>
                        {#if o.status.toLowerCase() === 'delivered' || o.status.toLowerCase() === 'completed'}
                          {#if hasReviewed(o.id, item.product_id)}
                            <span class="badge-reviewed">✓ Sudah Diulas</span>
                          {:else}
                            <button
                              class="btn-review-item"
                              onclick={() => openReviewModal(o.id, item.product_id, item.product_name)}
                            >
                              ⭐ Beri Ulasan
                            </button>
                          {/if}
                        {/if}
                      </div>
                      <span class="item-price">{formatRupiah(item.price_cents * item.quantity)}</span>
                    </div>
                  {/each}
                </div>

                <div class="box-foot">
                  <div>
                    <small>Total Bayar:</small>
                    <strong class="total">{formatRupiah(o.total_amount_cents)}</strong>
                  </div>
                  {#if o.status.toUpperCase() === 'SHIPPED'}
                    <button class="btn-confirm" onclick={() => confirmDelivery(o.id)}>
                      Konfirmasi Terima
                    </button>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

{#if isReviewModalOpen}
  <div
    class="modal-overlay review-overlay"
    onclick={() => (isReviewModalOpen = false)}
    role="button"
    tabindex="0"
    onkeydown={(e) => e.key === 'Escape' && (isReviewModalOpen = false)}
  >
    <div
      class="modal-card review-modal-card"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <div class="modal-header">
        <h3>⭐ Beri Ulasan & Rating</h3>
        <button class="close-btn" onclick={() => (isReviewModalOpen = false)}>&times;</button>
      </div>

      <div class="review-modal-body">
        <div class="target-product-banner">
          <small>Produk yang diulas:</small>
          <strong>{reviewTargetProductName}</strong>
        </div>

        <div class="form-group">
          <span class="rating-label" id="rating-group-label">Pilih Rating Bintang:</span>
          <div class="star-rating-selector" role="radiogroup" aria-labelledby="rating-group-label">
            {#each [1, 2, 3, 4, 5] as star}
              <button
                type="button"
                class="star-pick-btn"
                class:selected={star <= reviewRating}
                onclick={() => (reviewRating = star)}
                aria-label="{star} bintang"
                role="radio"
                aria-checked={reviewRating === star}
              >
                ★
              </button>
            {/each}
            <span class="star-value-text">{reviewRating} dari 5 bintang</span>
          </div>
        </div>

        <div class="form-group">
          <label for="review-textarea" class="text-label">Ulasan Pengalaman (opsional, maks 1000 karakter):</label>
          <textarea
            id="review-textarea"
            bind:value={reviewText}
            maxlength="1000"
            rows="4"
            placeholder="Ceritakan kepuasan Anda mengenai kualitas produk, pengiriman, dan pelayanan..."
          ></textarea>
          <div class="char-count">{reviewText.length} / 1000 karakter</div>
        </div>

        <div class="modal-footer-btns">
          <button type="button" class="btn-modal-cancel" onclick={() => (isReviewModalOpen = false)}>
            Batal
          </button>
          <button
            type="button"
            class="btn-modal-submit"
            onclick={submitReview}
            disabled={isSubmittingReview}
          >
            {isSubmittingReview ? 'Mengirim...' : 'Kirim Ulasan ⭐'}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.7);
    display: flex; align-items: center; justify-content: center; z-index: 1000;
  }
  .review-overlay {
    z-index: 1100;
  }
  .modal-card {
    background: #0f172a; border: 1px solid #334155; border-radius: 12px;
    width: 90%; max-width: 600px; max-height: 85vh; display: flex; flex-direction: column;
    padding: 1.5rem; color: #f8fafc; box-shadow: 0 10px 30px rgba(0,0,0,0.5);
  }
  .review-modal-card {
    max-width: 480px;
  }
  .modal-header {
    display: flex; justify-content: space-between; align-items: center;
    border-bottom: 1px solid #334155; padding-bottom: 0.75rem;
  }
  .modal-header h3 { margin: 0; font-size: 1.2rem; }
  .close-btn { background: none; border: none; font-size: 1.4rem; color: #94a3b8; cursor: pointer; }
  .orders-body { flex: 1; overflow-y: auto; margin-top: 1rem; padding-right: 0.25rem; }
  .status-msg, .empty-orders { text-align: center; color: #94a3b8; padding: 2rem 0; }
  .orders-list { display: flex; flex-direction: column; gap: 0.85rem; }
  .order-box {
    background: #1e293b; border: 1px solid #334155; border-radius: 8px; padding: 1rem;
  }
  .box-top { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem; }
  .oid { font-weight: bold; color: #38bdf8; margin-right: 0.5rem; font-family: monospace; }
  .date { color: #94a3b8; }
  .status-pill {
    padding: 0.2rem 0.5rem; border-radius: 4px; font-size: 0.75rem; font-weight: bold;
  }
  .status-pill.pending, .status-pill.pending_payment { background: #d97706; }
  .status-pill.paid, .status-pill.processing { background: #0284c7; }
  .status-pill.shipped { background: #7c3aed; }
  .status-pill.delivered, .status-pill.completed { background: #059669; }
  .status-pill.cancelled { background: #dc2626; }
  .items-summary { border-top: 1px solid #334155; border-bottom: 1px solid #334155; padding: 0.5rem 0; margin: 0.5rem 0; }
  .item-line { display: flex; justify-content: space-between; align-items: center; font-size: 0.85rem; margin-bottom: 0.4rem; }
  .item-info-col { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
  .item-name { font-weight: 500; }
  .item-price { color: #94a3b8; }
  .badge-reviewed {
    background: #064e3b; color: #34d399; font-size: 0.75rem; padding: 0.15rem 0.45rem;
    border-radius: 4px; font-weight: 600;
  }
  .btn-review-item {
    background: #4f46e5; color: #ffffff; border: none; font-size: 0.75rem; font-weight: 600;
    padding: 0.2rem 0.5rem; border-radius: 4px; cursor: pointer; transition: background 0.15s;
  }
  .btn-review-item:hover { background: #4338ca; }
  .box-foot { display: flex; justify-content: space-between; align-items: center; }
  .total { color: #38bdf8; margin-left: 0.25rem; }
  .btn-confirm {
    background: #059669; color: white; border: none; padding: 0.4rem 0.8rem;
    border-radius: 4px; font-weight: bold; cursor: pointer; font-size: 0.85rem;
  }

  /* Review Form Modal Styles */
  .review-modal-body { display: flex; flex-direction: column; gap: 1rem; margin-top: 1rem; }
  .target-product-banner {
    background: #1e293b; padding: 0.6rem 0.85rem; border-radius: 6px; border-left: 4px solid #38bdf8;
    display: flex; flex-direction: column;
  }
  .target-product-banner small { color: #94a3b8; font-size: 0.75rem; }
  .target-product-banner strong { font-size: 0.95rem; color: #f8fafc; }
  .form-group { display: flex; flex-direction: column; gap: 0.35rem; }
  .rating-label, .text-label { font-size: 0.85rem; color: #cbd5e1; font-weight: 600; }
  .star-rating-selector { display: flex; align-items: center; gap: 0.35rem; }
  .star-pick-btn {
    background: none; border: none; font-size: 1.8rem; color: #475569; cursor: pointer;
    line-height: 1; padding: 0.1rem; transition: color 0.15s, transform 0.1s;
  }
  .star-pick-btn.selected { color: #fbbf24; }
  .star-pick-btn:hover { transform: scale(1.15); }
  .star-value-text { margin-left: 0.5rem; font-size: 0.85rem; color: #94a3b8; }
  textarea {
    background: #1e293b; border: 1px solid #334155; border-radius: 6px; padding: 0.6rem;
    color: #f8fafc; font-family: inherit; font-size: 0.85rem; resize: vertical;
  }
  textarea:focus { outline: none; border-color: #38bdf8; }
  .char-count { text-align: right; font-size: 0.75rem; color: #94a3b8; }
  .modal-footer-btns { display: flex; justify-content: flex-end; gap: 0.75rem; margin-top: 0.5rem; }
  .btn-modal-cancel {
    background: #334155; color: #e2e8f0; border: none; padding: 0.5rem 1rem;
    border-radius: 6px; font-weight: 600; cursor: pointer;
  }
  .btn-modal-submit {
    background: #2563eb; color: #ffffff; border: none; padding: 0.5rem 1.25rem;
    border-radius: 6px; font-weight: 600; cursor: pointer; transition: background 0.15s;
  }
  .btn-modal-submit:hover:not(:disabled) { background: #1d4ed8; }
  .btn-modal-submit:disabled { opacity: 0.6; cursor: not-allowed; }
</style>
