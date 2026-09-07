<script lang="ts">
  import { formatRupiah } from './currency';
  import type { WishlistItem } from './types';

  interface Props {
    isOpen: boolean;
    onClose: () => void;
    wishlist: WishlistItem[];
    loading?: boolean;
    onRemoveItem: (productId: string) => Promise<void>;
    onAddToCart: (item: WishlistItem) => void;
  }

  let {
    isOpen,
    onClose,
    wishlist,
    loading = false,
    onRemoveItem,
    onAddToCart,
  }: Props = $props();

  let removingId = $state<string | null>(null);

  async function handleRemove(productId: string) {
    removingId = productId;
    try {
      await onRemoveItem(productId);
    } finally {
      removingId = null;
    }
  }
</script>

{#if isOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={onClose}>
    <div class="modal-card" onclick={(e) => e.stopPropagation()}>
      <div class="modal-header">
        <div class="header-title">
          <span class="heart-icon">❤️</span>
          <h3>Wishlist Saya</h3>
          {#if wishlist.length > 0}
            <span class="count-badge">{wishlist.length}</span>
          {/if}
        </div>
        <button class="close-btn" onclick={onClose} aria-label="Tutup modal">✕</button>
      </div>

      <div class="modal-body">
        {#if loading}
          <div class="loading-state">
            <div class="spinner"></div>
            <p>Memuat wishlist...</p>
          </div>
        {:else if wishlist.length === 0}
          <div class="empty-state">
            <div class="empty-icon">🤍</div>
            <h4>Wishlist Masih Kosong</h4>
            <p>Simpan produk favoritmu dengan menekan ikon hati (❤️) pada katalog belanja.</p>
            <button class="btn-browse" onclick={onClose}>Mulai Belanja</button>
          </div>
        {:else}
          <div class="wishlist-grid">
            {#each wishlist as item (item.id)}
              <div class="wishlist-card">
                <div class="card-img-wrapper">
                  {#if item.product_image_url}
                    <img src={item.product_image_url} alt={item.product_name} class="item-img" />
                  {:else}
                    <div class="img-placeholder">🛍️</div>
                  {/if}
                </div>

                <div class="card-info">
                  <h4 class="item-title" title={item.product_name}>{item.product_name}</h4>
                  <div class="item-price">{formatRupiah(item.product_price_cents)}</div>
                </div>

                <div class="card-actions">
                  <button
                    class="btn-cart"
                    onclick={() => onAddToCart(item)}
                    title="Tambah ke Keranjang"
                  >
                    🛒 + Keranjang
                  </button>
                  <button
                    class="btn-remove"
                    disabled={removingId === item.product_id}
                    onclick={() => handleRemove(item.product_id)}
                    title="Hapus dari wishlist"
                  >
                    {#if removingId === item.product_id}
                      ...
                    {:else}
                      🗑️
                    {/if}
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    padding: 1rem;
  }

  .modal-card {
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 16px;
    width: 100%;
    max-width: 680px;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.6);
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1.25rem 1.5rem;
    border-bottom: 1px solid #334155;
    background: #1e293b;
  }

  .header-title {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .heart-icon {
    font-size: 1.3rem;
  }

  .modal-header h3 {
    margin: 0;
    font-size: 1.2rem;
    font-weight: 700;
    color: #f8fafc;
  }

  .count-badge {
    background: #ef4444;
    color: #ffffff;
    font-size: 0.75rem;
    font-weight: 700;
    padding: 0.15rem 0.5rem;
    border-radius: 9999px;
  }

  .close-btn {
    background: none;
    border: none;
    font-size: 1.3rem;
    color: #94a3b8;
    cursor: pointer;
    padding: 0.25rem;
    border-radius: 6px;
    transition: color 0.15s, background 0.15s;
  }

  .close-btn:hover {
    color: #f8fafc;
    background: #334155;
  }

  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 1.25rem;
  }

  .loading-state,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 3rem 1.5rem;
    color: #94a3b8;
  }

  .empty-icon {
    font-size: 3rem;
    margin-bottom: 0.75rem;
  }

  .empty-state h4 {
    margin: 0 0 0.5rem 0;
    color: #f8fafc;
    font-size: 1.15rem;
  }

  .empty-state p {
    margin: 0 0 1.5rem 0;
    font-size: 0.9rem;
    max-width: 320px;
    line-height: 1.5;
  }

  .btn-browse {
    background: #0284c7;
    color: #ffffff;
    border: none;
    border-radius: 8px;
    padding: 0.65rem 1.25rem;
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s;
  }

  .btn-browse:hover {
    background: #0369a1;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid #334155;
    border-top-color: #38bdf8;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin-bottom: 0.75rem;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .wishlist-grid {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
  }

  .wishlist-card {
    display: flex;
    align-items: center;
    gap: 1rem;
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 10px;
    padding: 0.85rem 1rem;
    transition: border-color 0.15s;
  }

  .wishlist-card:hover {
    border-color: #475569;
  }

  .card-img-wrapper {
    width: 64px;
    height: 64px;
    min-width: 64px;
    border-radius: 8px;
    overflow: hidden;
    background: #0f172a;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .item-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .img-placeholder {
    font-size: 1.75rem;
  }

  .card-info {
    flex: 1;
    min-width: 0;
  }

  .item-title {
    margin: 0 0 0.35rem 0;
    font-size: 0.95rem;
    font-weight: 600;
    color: #f8fafc;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .item-price {
    font-size: 0.95rem;
    font-weight: 700;
    color: #38bdf8;
  }

  .card-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .btn-cart {
    background: #059669;
    color: #ffffff;
    border: none;
    border-radius: 6px;
    padding: 0.5rem 0.85rem;
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.15s;
  }

  .btn-cart:hover {
    background: #047857;
  }

  .btn-remove {
    background: #334155;
    color: #f87171;
    border: 1px solid #475569;
    border-radius: 6px;
    padding: 0.5rem 0.65rem;
    font-size: 0.85rem;
    cursor: pointer;
    transition: all 0.15s;
  }

  .btn-remove:hover:not(:disabled) {
    background: #dc2626;
    color: #ffffff;
    border-color: #dc2626;
  }

  .btn-remove:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  @media (max-width: 600px) {
    .wishlist-card {
      flex-direction: column;
      align-items: flex-start;
      gap: 0.75rem;
    }

    .card-actions {
      width: 100%;
      justify-content: flex-end;
    }

    .btn-cart {
      flex: 1;
      text-align: center;
    }
  }
</style>
