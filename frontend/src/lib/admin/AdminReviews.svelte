<script lang="ts">
  import { onMount } from 'svelte';
  import type { Product, ProductReview, PaginatedReviews } from '../types';
  import { toast } from '../toast.svelte';
  import { fetchAdmin } from './adminApi';
  import './admin-shared.css';

  interface Props {
    products: Product[];
  }

  let { products }: Props = $props();

  let adminReviews = $state<ProductReview[]>([]);
  let reviewsFilter = $state<'all' | 'visible' | 'hidden'>('all');
  let adminReviewsPage = $state(1);
  let adminReviewsPageSize = $state(20);
  let adminReviewsTotal = $state(0);
  let adminReviewsTotalPages = $state(1);
  let reviewsLoading = $state(false);

  async function loadAdminReviews(page = 1) {
    reviewsLoading = true;
    try {
      let url = `/api/v1/admin/reviews?page=${page}&page_size=${adminReviewsPageSize}`;
      if (reviewsFilter !== 'all') {
        url += `&status=${reviewsFilter}`;
      }
      const revs = await fetchAdmin<PaginatedReviews>(url);
      adminReviews = revs.items || [];
      adminReviewsTotal = revs.total;
      adminReviewsPage = revs.page;
      adminReviewsTotalPages = revs.total_pages;
    } catch (err: any) {
      toast.error(err.message || 'Gagal memuat ulasan');
    } finally {
      reviewsLoading = false;
    }
  }

  function setReviewsFilter(filter: 'all' | 'visible' | 'hidden') {
    reviewsFilter = filter;
    adminReviewsPage = 1;
    loadAdminReviews(1);
  }

  function changeAdminReviewsPage(newPage: number) {
    if (newPage < 1 || newPage > adminReviewsTotalPages) return;
    loadAdminReviews(newPage);
  }

  async function handleToggleReviewVisibility(review: ProductReview) {
    const newVisibility = !review.is_visible;
    try {
      const updated = await fetchAdmin<ProductReview>(`/api/v1/admin/reviews/${review.id}/visibility`, {
        method: 'PATCH',
        body: JSON.stringify({ is_visible: newVisibility }),
      });
      if (reviewsFilter !== 'all') {
        await loadAdminReviews(adminReviewsPage);
      } else {
        adminReviews = adminReviews.map(r => r.id === review.id ? updated : r);
      }
      toast.success(`Ulasan berhasil ${newVisibility ? 'ditampilkan' : 'disembunyikan'}!`);
    } catch (err: any) {
      toast.error(err.message || 'Gagal mengubah status visibilitas ulasan');
    }
  }

  onMount(() => {
    loadAdminReviews(1);
  });
</script>

<div class="section-panel">
  <div class="panel-top">
    <div>
      <h2>⭐ Moderasi Ulasan & Rating Produk</h2>
      <span class="panel-subtitle">Total: <strong>{adminReviewsTotal}</strong> ulasan &bull; Halaman {adminReviewsPage} dari {adminReviewsTotalPages}</span>
    </div>
    <div class="filter-group">
      <button
        class="btn-filter"
        class:active={reviewsFilter === 'all'}
        onclick={() => setReviewsFilter('all')}
      >
        Semua
      </button>
      <button
        class="btn-filter"
        class:active={reviewsFilter === 'visible'}
        onclick={() => setReviewsFilter('visible')}
      >
        Tampil
      </button>
      <button
        class="btn-filter"
        class:active={reviewsFilter === 'hidden'}
        onclick={() => setReviewsFilter('hidden')}
      >
        Disembunyikan
      </button>
    </div>
  </div>

  {#if reviewsLoading}
    <p class="empty-text">Memuat daftar ulasan...</p>
  {:else if adminReviews.length === 0}
    <p class="empty-text">Tidak ada ulasan ditemukan.</p>
  {:else}
    <div style="overflow-x: auto;">
      <table class="table-custom">
        <thead>
          <tr>
            <th>Produk</th>
            <th>Pembeli</th>
            <th>Rating</th>
            <th>Isi Ulasan</th>
            <th>Tanggal</th>
            <th>Status</th>
            <th>Aksi Moderasi</th>
          </tr>
        </thead>
        <tbody>
          {#each adminReviews as rev (rev.id)}
            <tr>
              <td>
                <strong>{products.find(p => p.id === rev.product_id)?.name || 'Produk'}</strong>
                <div><small><code>#{rev.product_id.slice(0, 8)}</code></small></div>
              </td>
              <td>
                <strong>{rev.buyer_name}</strong>
              </td>
              <td>
                <span class="stars-badge">
                  {"★".repeat(rev.rating)}{"☆".repeat(5 - rev.rating)} ({rev.rating}/5)
                </span>
              </td>
              <td class="review-text-cell">
                {#if rev.review_text}
                  <span>{rev.review_text}</span>
                {:else}
                  <em class="text-muted">(Tanpa teks ulasan)</em>
                {/if}
              </td>
              <td>
                <small>{new Date(rev.created_at).toLocaleDateString('id-ID')}</small>
              </td>
              <td>
                <span class="status-pill {rev.is_visible ? 'delivered' : 'cancelled'}">
                  {rev.is_visible ? '✓ Tampil' : '✕ Disembunyikan'}
                </span>
              </td>
              <td>
                <button
                  class="btn-action"
                  class:primary={!rev.is_visible}
                  onclick={() => handleToggleReviewVisibility(rev)}
                >
                  {rev.is_visible ? 'Sembunyikan' : 'Tampilkan'}
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    {#if adminReviewsTotalPages > 1}
      <div class="pagination-bar">
        <button
          class="btn-page"
          disabled={adminReviewsPage <= 1 || reviewsLoading}
          onclick={() => changeAdminReviewsPage(adminReviewsPage - 1)}
        >
          &larr; Sebelumnya
        </button>
        <span class="page-info">
          Halaman {adminReviewsPage} dari {adminReviewsTotalPages} ({adminReviewsTotal} ulasan)
        </span>
        <button
          class="btn-page"
          disabled={adminReviewsPage >= adminReviewsTotalPages || reviewsLoading}
          onclick={() => changeAdminReviewsPage(adminReviewsPage + 1)}
        >
          Selanjutnya &rarr;
        </button>
      </div>
    {/if}
  {/if}
</div>

<style>
  .section-panel { width: 100%; }
  h2 { margin: 0; font-size: 1.25rem; color: #f8fafc; }
  .panel-top { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
  .panel-subtitle { font-size: 0.85rem; color: #94a3b8; margin-top: 0.25rem; display: inline-block; }
  .filter-group { display: flex; gap: 0.5rem; }
  .btn-filter {
    background: #1e293b; color: #94a3b8; border: 1px solid #334155;
    padding: 0.35rem 0.75rem; border-radius: 6px; cursor: pointer; font-size: 0.8rem;
    transition: all 0.2s;
  }
  .btn-filter:hover { background: #334155; color: #fff; }
  .btn-filter.active { background: #0284c7; color: #fff; border-color: #0284c7; font-weight: bold; }
  .stars-badge { color: #f59e0b; font-size: 0.85rem; font-weight: 500; }
  .review-text-cell { max-width: 320px; font-size: 0.85rem; line-height: 1.4; color: #cbd5e1; }
  .pagination-bar {
    display: flex; justify-content: space-between; align-items: center;
    padding-top: 1rem; margin-top: 1rem; border-top: 1px solid #334155;
  }
  .btn-page {
    background: #1e293b; color: #cbd5e1; border: 1px solid #334155;
    padding: 0.4rem 0.85rem; border-radius: 6px; font-size: 0.8rem; cursor: pointer;
  }
  .btn-page:hover:not(:disabled) { background: #334155; color: #fff; }
  .btn-page:disabled { opacity: 0.4; cursor: not-allowed; }
  .page-info { font-size: 0.8rem; color: #94a3b8; }
</style>
