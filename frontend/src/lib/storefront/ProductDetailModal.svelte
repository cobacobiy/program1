<script lang="ts">
  import type { Product, ProductRatingSummary, PublicReview } from '../types';
  import { formatRupiah } from '../currency';
  import { i18n } from '../i18n.svelte';

  interface Props {
    product: Product | null;
    activeRatingSummary: ProductRatingSummary | null;
    activeReviews: PublicReview[];
    reviewsLoading: boolean;
    reviewPage: number;
    reviewTotalPages: number;
    reviewTotal: number;
    recentlyAddedId: string | null;
    isWishlisted: boolean;
    onClose: () => void;
    onAddToCart: (product: Product) => void;
    onToggleWishlist: (product: Product) => void;
    onChangeReviewPage: (page: number) => void;
  }

  let {
    product,
    activeRatingSummary,
    activeReviews,
    reviewsLoading,
    reviewPage,
    reviewTotalPages,
    reviewTotal,
    recentlyAddedId,
    isWishlisted,
    onClose,
    onAddToCart,
    onToggleWishlist,
    onChangeReviewPage,
  }: Props = $props();
</script>

{#if product}
  <div
    class="drawer-backdrop"
    onclick={onClose}
    role="presentation"
  >
    <div
      class="modal-card product-detail-modal"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <div class="modal-header">
        <div>
          <h3>{product.name}</h3>
          <span class="badge-tag">{product.category || 'Umum'}</span>
        </div>
        <button class="close-btn" onclick={onClose}>&times;</button>
      </div>

      <div class="detail-body">
        <!-- Overview: image + details -->
        <div class="detail-overview">
          <div class="detail-img-box">
            {#if product.image_url}
              <img src={product.image_url} alt={product.name} />
            {:else}
              <span class="placeholder-emoji large">📦</span>
            {/if}
          </div>

          <div class="detail-info">
            <div class="detail-price-line">
              <span class="detail-price">{formatRupiah(product.price_cents)}</span>
              <span class="detail-stock" class:out={product.stock <= 0}>
                {product.stock > 0 ? `${i18n.t('product.stock_available', 'Stok')}: ${product.stock} ${i18n.t('catalog.units', 'unit')}` : i18n.t('catalog.out_of_stock', 'Stok Habis')}
              </span>
            </div>
            <p class="detail-desc">{product.description || '-'}</p>

            <div class="detail-actions-row">
              <button
                class="btn-add-detail"
                class:added={recentlyAddedId === product.id}
                disabled={product.stock <= 0}
                onclick={() => onAddToCart(product)}
                type="button"
              >
                {#if recentlyAddedId === product.id}
                  ✓ {i18n.t('catalog.added', 'Berhasil Ditambahkan ke Keranjang!')}
                {:else if product.stock <= 0}
                  {i18n.t('catalog.out_of_stock', 'Stok Habis')}
                {:else}
                  🛒 {i18n.t('catalog.buy_now_long', 'Beli Sekarang / Masukkan ke Keranjang')}
                {/if}
              </button>
              <button
                class="btn-fav-detail"
                class:active={isWishlisted}
                onclick={() => onToggleWishlist(product)}
                type="button"
              >
                {isWishlisted ? '❤️ Wishlist' : '🤍 Wishlist'}
              </button>
            </div>
          </div>
        </div>

        <!-- Reviews & Rating Section -->
        <div class="reviews-section">
          <h4 class="reviews-heading">⭐ {i18n.t('product.reviews_title', 'Ulasan & Penilaian Pembeli')}</h4>

          <!-- Rating Summary Grid -->
          <div class="rating-summary-container">
            <!-- Overall Score Column -->
            <div class="score-box">
              <span class="big-score">
                {activeRatingSummary ? activeRatingSummary.average_rating.toFixed(1) : '0.0'}
              </span>
              <div class="star-row">
                {#if activeRatingSummary && activeRatingSummary.average_rating > 0}
                  {"★".repeat(Math.round(activeRatingSummary.average_rating))}{"☆".repeat(5 - Math.round(activeRatingSummary.average_rating))}
                {:else}
                  ☆☆☆☆☆
                {/if}
              </div>
              <small class="total-rev-text">
                {activeRatingSummary ? activeRatingSummary.total_reviews : 0} {i18n.t('catalog.rating', 'ulasan')}
              </small>
            </div>

            <!-- 1-5 Star Breakdown Bar Chart -->
            <div class="breakdown-box">
              {#each [5, 4, 3, 2, 1] as star}
                {@const count = activeRatingSummary?.rating_distribution ? activeRatingSummary.rating_distribution[star - 1] : 0}
                {@const total = activeRatingSummary?.total_reviews || 0}
                {@const pct = total > 0 ? Math.round((count / total) * 100) : 0}
                <div class="breakdown-row">
                  <span class="star-label">{star} ★</span>
                  <div class="progress-track">
                    <div class="progress-fill" style="width: {pct}%;"></div>
                  </div>
                  <span class="count-label">{count} ({pct}%)</span>
                </div>
              {/each}
            </div>
          </div>

          <!-- Review Cards List -->
          <div class="reviews-list-container">
            {#if reviewsLoading}
              <p class="loading-revs">{i18n.t('common.loading', 'Memuat ulasan...')}</p>
            {:else if activeReviews.length === 0}
              <div class="empty-reviews">
                <p>{i18n.t('product.no_reviews', 'Belum ada ulasan untuk produk ini.')}</p>
              </div>
            {:else}
              <div class="reviews-cards">
                {#each activeReviews as r (r.id)}
                  <div class="review-card">
                    <div class="rev-header">
                      <div class="rev-user">
                        <span class="user-avatar">👤</span>
                        <div>
                          <strong class="user-name">{r.buyer_name}</strong>
                          <div class="rev-stars">
                            {"★".repeat(r.rating)}{"☆".repeat(5 - r.rating)}
                          </div>
                        </div>
                      </div>
                      <span class="rev-date">{new Date(r.created_at).toLocaleDateString(i18n.current === 'id' ? 'id-ID' : 'en-US')}</span>
                    </div>

                    {#if r.review_text}
                      <p class="rev-comment">{r.review_text}</p>
                    {/if}
                  </div>
                {/each}
              </div>

              <!-- Pagination -->
              {#if reviewTotalPages > 1}
                <div class="pagination-bar">
                  <button
                    class="btn-page"
                    disabled={reviewPage <= 1}
                    onclick={() => onChangeReviewPage(reviewPage - 1)}
                  >
                    &larr; Sebelumnya
                  </button>
                  <span class="page-info">
                    Halaman {reviewPage} dari {reviewTotalPages} ({reviewTotal} ulasan)
                  </span>
                  <button
                    class="btn-page"
                    disabled={reviewPage >= reviewTotalPages}
                    onclick={() => onChangeReviewPage(reviewPage + 1)}
                  >
                    Selanjutnya &rarr;
                  </button>
                </div>
              {/if}
            {/if}
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .drawer-backdrop {
    position: fixed; inset: 0; background: rgba(0,0,0,0.6); z-index: 300;
    display: flex; align-items: center; justify-content: center;
  }
  .modal-card {
    background: #0f172a; border: 1px solid #334155; border-radius: 12px;
    box-shadow: 0 10px 35px rgba(0,0,0,0.7); overflow: hidden;
  }
  .modal-header {
    padding: 1.25rem; border-bottom: 1px solid #1e293b;
    display: flex; justify-content: space-between; align-items: flex-start;
  }
  .modal-header h3 { margin: 0; }
  .badge-tag {
    display: inline-block; background: #334155; color: #38bdf8;
    font-size: 0.75rem; padding: 0.2rem 0.5rem; border-radius: 4px; margin-top: 0.25rem;
  }
  .close-btn {
    background: none; border: none; font-size: 1.5rem; color: #94a3b8; cursor: pointer;
  }
  .product-detail-modal {
    width: 90%; max-width: 640px; max-height: 88vh; display: flex; flex-direction: column;
    color: #f8fafc;
  }
  .detail-body { padding: 1.25rem; overflow-y: auto; display: flex; flex-direction: column; gap: 1.5rem; }
  .detail-overview { display: flex; gap: 1.25rem; }
  .detail-img-box {
    width: 140px; height: 140px; border-radius: 8px; background: #1e293b;
    display: flex; align-items: center; justify-content: center; overflow: hidden; flex-shrink: 0;
  }
  .detail-img-box img { width: 100%; height: 100%; object-fit: cover; }
  .placeholder-emoji.large { font-size: 3rem; }
  .detail-info { display: flex; flex-direction: column; gap: 0.6rem; flex: 1; }
  .detail-price-line { display: flex; align-items: baseline; gap: 0.75rem; }
  .detail-price { color: #38bdf8; font-size: 1.35rem; font-weight: bold; }
  .detail-stock { font-size: 0.85rem; color: #10b981; font-weight: 500; }
  .detail-stock.out { color: #ef4444; }
  .detail-desc { font-size: 0.88rem; color: #cbd5e1; line-height: 1.45; margin: 0; }
  .detail-actions-row { display: flex; flex-direction: column; gap: 0.5rem; margin-top: auto; }
  .btn-add-detail {
    position: relative;
    overflow: hidden;
    background: linear-gradient(135deg, #0284c7 0%, #0369a1 100%);
    color: #fff;
    border: none;
    padding: 0.75rem 1.25rem;
    border-radius: 8px;
    font-weight: 700;
    font-size: 0.95rem;
    cursor: pointer;
    box-shadow: 0 2px 8px rgba(2, 132, 199, 0.35);
    transition: transform 0.15s cubic-bezier(0.34, 1.56, 0.64, 1),
                box-shadow 0.15s ease,
                background 0.25s ease,
                filter 0.15s ease;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    user-select: none;
    -webkit-tap-highlight-color: transparent;
  }
  .btn-add-detail:hover:not(:disabled) {
    background: linear-gradient(135deg, #0369a1 0%, #075985 100%);
    transform: translateY(-2px);
    box-shadow: 0 5px 16px rgba(2, 132, 199, 0.45);
    filter: brightness(1.08);
  }
  .btn-add-detail:active:not(:disabled) {
    transform: translateY(2px) scale(0.94);
    box-shadow: 0 1px 3px rgba(2, 132, 199, 0.2);
    filter: brightness(0.9);
    transition: transform 0.06s ease;
  }
  .btn-add-detail.added {
    background: linear-gradient(135deg, #10b981 0%, #059669 100%) !important;
    box-shadow: 0 4px 16px rgba(16, 185, 129, 0.5) !important;
    transform: scale(1.03);
    animation: buttonPop 0.35s cubic-bezier(0.175, 0.885, 0.32, 1.275);
  }
  .btn-add-detail:disabled {
    background: #475569;
    box-shadow: none;
    cursor: not-allowed;
    transform: none;
  }
  .btn-fav-detail {
    background: #1e293b; color: #f8fafc; border: 1px solid #334155;
    padding: 0.6rem 1rem; border-radius: 6px; font-weight: 600; font-size: 0.88rem;
    cursor: pointer; transition: all 0.2s; width: 100%;
  }
  .btn-fav-detail:hover {
    background: #334155; border-color: #ef4444;
  }
  .btn-fav-detail.active {
    background: #450a0a; border-color: #ef4444; color: #fca5a5;
  }
  .reviews-section { border-top: 1px solid #1e293b; padding-top: 1.25rem; }
  .reviews-heading { margin: 0 0 1rem; font-size: 1.1rem; color: #f1f5f9; }
  .rating-summary-container {
    display: flex; gap: 1.5rem; background: #1e293b; padding: 1rem 1.25rem;
    border-radius: 10px; border: 1px solid #334155; margin-bottom: 1.25rem;
    align-items: center;
  }
  .score-box {
    display: flex; flex-direction: column; align-items: center; justify-content: center;
    min-width: 110px; border-right: 1px solid #334155; padding-right: 1rem;
  }
  .big-score { font-size: 2.4rem; font-weight: 900; color: #f59e0b; line-height: 1; }
  .star-row { color: #f59e0b; font-size: 1.1rem; margin: 0.35rem 0 0.2rem; }
  .total-rev-text { font-size: 0.75rem; color: #94a3b8; text-align: center; }
  .breakdown-box { flex: 1; display: flex; flex-direction: column; gap: 0.35rem; }
  .breakdown-row { display: flex; align-items: center; gap: 0.5rem; font-size: 0.8rem; }
  .star-label { width: 30px; color: #cbd5e1; font-weight: 500; text-align: right; }
  .progress-track {
    flex: 1; height: 8px; background: #0f172a; border-radius: 4px; overflow: hidden;
  }
  .progress-fill { height: 100%; background: #f59e0b; border-radius: 4px; transition: width 0.3s ease; }
  .count-label { width: 60px; color: #94a3b8; font-size: 0.75rem; text-align: right; }
  .reviews-list-container { display: flex; flex-direction: column; gap: 0.85rem; }
  .loading-revs { text-align: center; color: #94a3b8; padding: 1.5rem; }
  .empty-reviews {
    text-align: center; padding: 1.5rem 1rem; background: #1e293b; border-radius: 8px;
    color: #94a3b8;
  }
  .empty-reviews p { margin: 0 0 0.25rem; font-weight: 500; color: #cbd5e1; }
  .reviews-cards { display: flex; flex-direction: column; gap: 0.75rem; }
  .review-card {
    background: #1e293b; border: 1px solid #334155; border-radius: 8px;
    padding: 0.85rem 1rem; display: flex; flex-direction: column; gap: 0.5rem;
  }
  .rev-header { display: flex; justify-content: space-between; align-items: flex-start; }
  .rev-user { display: flex; align-items: center; gap: 0.6rem; }
  .user-avatar { font-size: 1.3rem; }
  .user-name { font-size: 0.9rem; color: #f1f5f9; display: block; }
  .rev-stars { color: #f59e0b; font-size: 0.85rem; }
  .rev-date { font-size: 0.75rem; color: #64748b; }
  .rev-comment {
    margin: 0; font-size: 0.85rem; color: #cbd5e1; line-height: 1.4;
    white-space: pre-wrap; word-break: break-word;
  }
  .pagination-bar {
    display: flex; justify-content: space-between; align-items: center;
    padding-top: 0.5rem; margin-top: 0.5rem;
  }
  .btn-page {
    background: #1e293b; color: #cbd5e1; border: 1px solid #334155;
    padding: 0.4rem 0.85rem; border-radius: 6px; font-size: 0.8rem; cursor: pointer;
  }
  .btn-page:hover:not(:disabled) { background: #334155; color: #fff; }
  .btn-page:disabled { opacity: 0.4; cursor: not-allowed; }
  .page-info { font-size: 0.8rem; color: #94a3b8; }

  @keyframes buttonPop {
    0% { transform: scale(0.92); }
    50% { transform: scale(1.1); }
    100% { transform: scale(1.04); }
  }
</style>
