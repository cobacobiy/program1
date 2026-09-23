<script lang="ts">
  import type { Product, ProductRatingSummary } from '../types';
  import { formatRupiah } from '../currency';
  import { i18n } from '../i18n.svelte';

  interface Props {
    products: Product[];
    catalogLoading: boolean;
    catalogError: string | null;
    searchQuery: string;
    ratingSummaries: Record<string, ProductRatingSummary>;
    isProductInWishlist: (productId: string) => boolean;
    recentlyAddedId: string | null;
    onRetryFetch: () => void;
    onOpenProductDetail: (product: Product) => void;
    onToggleWishlist: (product: Product) => void;
    onAddToCart: (product: Product) => void;
  }

  let {
    products,
    catalogLoading,
    catalogError,
    searchQuery,
    ratingSummaries,
    isProductInWishlist,
    recentlyAddedId,
    onRetryFetch,
    onOpenProductDetail,
    onToggleWishlist,
    onAddToCart,
  }: Props = $props();
</script>

{#if catalogLoading}
  <div class="grid-skeleton">
    {#each Array(6) as _}
      <div class="card-skeleton"></div>
    {/each}
  </div>
{:else if catalogError}
  <div class="alert-box error">
    <p><strong>{i18n.t('catalog.load_failed', 'Gagal Memuat Produk')}:</strong> {catalogError}</p>
    <button class="btn-retry" onclick={onRetryFetch}>{i18n.t('catalog.retry', 'Coba Lagi')}</button>
  </div>
{:else if products.length === 0}
  <div class="empty-box">
    <p>{i18n.t('catalog.empty_search', 'Produk tidak ditemukan untuk pencarian')} "{searchQuery}".</p>
  </div>
{:else}
  <div class="catalog-grid">
    {#each products as product (product.id)}
      <div class="product-item">
        <div
          class="img-container clickable"
          onclick={() => onOpenProductDetail(product)}
          role="button"
          tabindex="0"
          onkeydown={(e) => e.key === 'Enter' && onOpenProductDetail(product)}
        >
          {#if product.image_url}
            <img src={product.image_url} alt={product.name} loading="lazy" />
          {:else}
            <span class="placeholder-emoji">📦</span>
          {/if}
          {#if product.stock <= 0}
            <span class="badge-habis">{i18n.t('catalog.out_of_stock', 'Habis')}</span>
          {/if}
          <button
            class="btn-fav-toggle"
            class:active={isProductInWishlist(product.id)}
            onclick={(e) => { e.stopPropagation(); onToggleWishlist(product); }}
            title={isProductInWishlist(product.id) ? "Hapus dari wishlist" : "Tambah ke wishlist"}
            type="button"
            aria-label="Wishlist"
          >
            {isProductInWishlist(product.id) ? '❤️' : '🤍'}
          </button>
        </div>

        <div class="item-details">
          <h3 class="p-title">
            <button
              type="button"
              class="p-title-btn"
              onclick={() => onOpenProductDetail(product)}
            >
              {product.name}
            </button>
          </h3>

          <!-- Rating Preview -->
          <button
            class="rating-badge-btn"
            onclick={() => onOpenProductDetail(product)}
            type="button"
            title="Lihat ulasan produk"
          >
            {#if ratingSummaries[product.id] && ratingSummaries[product.id].total_reviews > 0}
              <span class="star-icon">⭐</span>
              <strong class="rating-score">{ratingSummaries[product.id].average_rating.toFixed(1)}</strong>
              <span class="review-count">({ratingSummaries[product.id].total_reviews})</span>
            {:else}
              <span class="rating-none">☆ {i18n.t('product.no_reviews', 'Belum ada ulasan')}</span>
            {/if}
          </button>

          <p class="p-desc">{product.description || 'Produk kualitas terjamin dari katalog.'}</p>
          
          <div class="item-meta">
            <span class="price-value">{formatRupiah(product.price_cents)}</span>
            <small class="stock-value">{i18n.t('product.stock_available', 'Stok')}: {product.stock}</small>
          </div>

          <div class="card-actions-row">
            <button
              class="btn-detail"
              onclick={() => onOpenProductDetail(product)}
              type="button"
            >
              {i18n.t('catalog.view_detail', 'Detail & Ulasan')}
            </button>
            <button
              class="btn-add"
              class:added={recentlyAddedId === product.id}
              disabled={product.stock <= 0}
              onclick={() => onAddToCart(product)}
              type="button"
            >
              {#if recentlyAddedId === product.id}
                ✓ {i18n.t('catalog.added', 'Ditambahkan!')}
              {:else if product.stock <= 0}
                {i18n.t('catalog.out_of_stock', 'Habis')}
              {:else}
                + {i18n.t('catalog.buy_now', 'Beli')}
              {/if}
            </button>
          </div>
        </div>
      </div>
    {/each}
  </div>
{/if}

<style>
  .grid-skeleton {
    display: grid; grid-template-columns: repeat(auto-fill, minmax(230px, 1fr)); gap: 1.5rem;
  }
  .card-skeleton {
    height: 340px; background: #1e293b; border-radius: 12px;
    animation: pulse 1.5s infinite ease-in-out;
  }
  @keyframes pulse {
    0%, 100% { opacity: 0.6; }
    50% { opacity: 0.3; }
  }
  .alert-box { padding: 1.25rem; border-radius: 8px; margin-top: 1rem; }
  .alert-box.error { background: #7f1d1d; color: #fecaca; }
  .btn-retry {
    background: #dc2626; color: #fff; border: none; padding: 0.4rem 0.8rem;
    border-radius: 4px; margin-top: 0.5rem; cursor: pointer;
  }
  .empty-box {
    text-align: center; padding: 3rem; background: #1e293b; border-radius: 12px;
    color: #94a3b8;
  }
  .catalog-grid {
    display: grid; grid-template-columns: repeat(auto-fill, minmax(230px, 1fr)); gap: 1.5rem;
  }
  .product-item {
    background: #1e293b; border: 1px solid #334155; border-radius: 12px;
    overflow: hidden; display: flex; flex-direction: column;
    transition: transform 0.2s, box-shadow 0.2s, border-color 0.2s;
  }
  .product-item:hover {
    transform: translateY(-4px); box-shadow: 0 8px 24px rgba(0,0,0,0.4); border-color: #0284c7;
  }
  .img-container {
    height: 180px; background: #0f172a; position: relative;
    display: flex; align-items: center; justify-content: center;
  }
  .img-container img { width: 100%; height: 100%; object-fit: cover; }
  .placeholder-emoji { font-size: 3.5rem; }
  .badge-habis {
    position: absolute; top: 8px; right: 8px; background: #dc2626; color: #fff;
    font-size: 0.75rem; font-weight: bold; padding: 0.2rem 0.5rem; border-radius: 4px;
  }
  .btn-fav-toggle {
    position: absolute; top: 8px; left: 8px;
    background: rgba(15, 23, 42, 0.75); backdrop-filter: blur(4px);
    border: 1px solid rgba(255, 255, 255, 0.15); border-radius: 50%;
    width: 34px; height: 34px; display: flex; align-items: center; justify-content: center;
    font-size: 1rem; cursor: pointer; transition: transform 0.15s, background 0.15s;
    z-index: 2;
  }
  .btn-fav-toggle:hover {
    transform: scale(1.15); background: rgba(15, 23, 42, 0.95);
  }
  .btn-fav-toggle.active {
    border-color: #ef4444; background: rgba(239, 68, 68, 0.25);
  }
  .item-details { padding: 1.25rem; flex: 1; display: flex; flex-direction: column; }
  .p-title { margin: 0 0 0.5rem; font-size: 1.05rem; color: #f8fafc; }
  .p-title-btn {
    background: none;
    border: none;
    padding: 0;
    font: inherit;
    color: inherit;
    cursor: pointer;
    text-align: left;
    width: 100%;
    transition: color 0.15s ease;
  }
  .p-title-btn:hover {
    color: #38bdf8;
  }
  .rating-badge-btn {
    background: transparent; border: none; padding: 0.2rem 0;
    cursor: pointer; display: flex; align-items: center; gap: 0.35rem;
    font-size: 0.85rem; text-align: left;
  }
  .rating-badge-btn:hover .rating-score { text-decoration: underline; }
  .star-icon { font-size: 0.9rem; }
  .rating-score { color: #f59e0b; font-weight: bold; }
  .review-count { color: #94a3b8; font-size: 0.8rem; }
  .rating-none { color: #64748b; font-size: 0.78rem; font-style: italic; }
  .p-desc { font-size: 0.85rem; color: #94a3b8; margin: 0 0 1rem; flex: 1; }
  .item-meta { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
  .price-value { font-size: 1.15rem; font-weight: bold; color: #38bdf8; }
  .stock-value { color: #64748b; }
  .card-actions-row { display: flex; gap: 0.5rem; width: 100%; margin-top: 0.5rem; }
  .btn-detail {
    flex: 1; background: #1e293b; color: #cbd5e1; border: 1px solid #334155;
    padding: 0.6rem 0.5rem; border-radius: 6px; font-size: 0.8rem; font-weight: 500;
    cursor: pointer; transition: all 0.2s;
  }
  .btn-detail:hover { background: #334155; color: #fff; }
  .btn-add {
    position: relative;
    overflow: hidden;
    background: linear-gradient(135deg, #0284c7 0%, #0369a1 100%);
    color: #fff;
    border: none;
    padding: 0.65rem 0.95rem;
    border-radius: 8px;
    font-weight: 600;
    font-size: 0.88rem;
    cursor: pointer;
    box-shadow: 0 2px 6px rgba(2, 132, 199, 0.35);
    transition: transform 0.15s cubic-bezier(0.34, 1.56, 0.64, 1),
                box-shadow 0.15s ease,
                background 0.25s ease,
                filter 0.15s ease;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.35rem;
    user-select: none;
    -webkit-tap-highlight-color: transparent;
  }
  .btn-add:hover:not(:disabled) {
    background: linear-gradient(135deg, #0369a1 0%, #075985 100%);
    transform: translateY(-2px);
    box-shadow: 0 4px 14px rgba(2, 132, 199, 0.45);
    filter: brightness(1.08);
  }
  .btn-add:active:not(:disabled) {
    transform: translateY(2px) scale(0.92);
    box-shadow: 0 1px 2px rgba(2, 132, 199, 0.2);
    filter: brightness(0.9);
    transition: transform 0.06s ease;
  }
  .btn-add.added {
    background: linear-gradient(135deg, #10b981 0%, #059669 100%) !important;
    box-shadow: 0 4px 16px rgba(16, 185, 129, 0.5) !important;
    transform: scale(1.04);
    animation: buttonPop 0.35s cubic-bezier(0.175, 0.885, 0.32, 1.275);
  }
  .btn-add:disabled {
    background: #475569;
    box-shadow: none;
    cursor: not-allowed;
    transform: none;
  }
  .clickable { cursor: pointer; }
  .clickable:hover { opacity: 0.9; }
  @keyframes buttonPop {
    0% { transform: scale(0.92); }
    50% { transform: scale(1.1); }
    100% { transform: scale(1.04); }
  }
</style>
