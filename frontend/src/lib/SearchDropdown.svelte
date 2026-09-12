<script lang="ts">
  import { onMount } from 'svelte';
  import { apiFetch } from './api';
  import { formatRupiah } from './currency';
  import type { ProductSuggestionItem, PopularSearchKeyword, SearchSuggestionResult } from './types';

  interface Props {
    query: string;
    isOpen: boolean;
    onSelectProduct: (product: ProductSuggestionItem) => void;
    onSelectKeyword: (keyword: string) => void;
    onClose: () => void;
  }

  let {
    query = $bindable(''),
    isOpen = $bindable(false),
    onSelectProduct,
    onSelectKeyword,
    onClose,
  }: Props = $props();

  const RECENT_KEY = 'program1_recent_searches';

  let recentSearches = $state<string[]>([]);
  let popularKeywords = $state<PopularSearchKeyword[]>([]);
  let productSuggestions = $state<ProductSuggestionItem[]>([]);
  let categorySuggestions = $state<string[]>([]);
  let isLoading = $state(false);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let selectedIndex = $state<number>(-1);

  onMount(() => {
    loadRecentSearches();
    fetchPopularSearches();
  });

  function loadRecentSearches() {
    try {
      const stored = localStorage.getItem(RECENT_KEY);
      if (stored) {
        recentSearches = JSON.parse(stored);
      }
    } catch {
      recentSearches = [];
    }
  }

  export function saveRecentSearch(keyword: string) {
    const trimmed = keyword.trim();
    if (!trimmed) return;
    try {
      const filtered = recentSearches.filter((item) => item.toLowerCase() !== trimmed.toLowerCase());
      const updated = [trimmed, ...filtered].slice(0, 8);
      recentSearches = updated;
      localStorage.setItem(RECENT_KEY, JSON.stringify(updated));
    } catch {
      // ignore
    }
  }

  function removeRecentSearch(keyword: string, event: MouseEvent) {
    event.stopPropagation();
    recentSearches = recentSearches.filter((item) => item !== keyword);
    try {
      localStorage.setItem(RECENT_KEY, JSON.stringify(recentSearches));
    } catch {
      // ignore
    }
  }

  function clearAllRecentSearches() {
    recentSearches = [];
    try {
      localStorage.removeItem(RECENT_KEY);
    } catch {
      // ignore
    }
  }

  async function fetchPopularSearches() {
    try {
      const res = await apiFetch<PopularSearchKeyword[]>('/api/v1/catalog/popular-searches?limit=6');
      popularKeywords = res || [];
    } catch {
      popularKeywords = [];
    }
  }

  $effect(() => {
    const trimmed = query.trim();
    if (debounceTimer) clearTimeout(debounceTimer);

    if (trimmed.length === 0) {
      productSuggestions = [];
      categorySuggestions = [];
      isLoading = false;
      return;
    }

    isLoading = true;
    debounceTimer = setTimeout(async () => {
      try {
        const res = await apiFetch<SearchSuggestionResult>(
          `/api/v1/catalog/suggest?q=${encodeURIComponent(trimmed)}&limit=6`
        );
        productSuggestions = res.product_suggestions || [];
        categorySuggestions = res.category_suggestions || [];
      } catch {
        productSuggestions = [];
        categorySuggestions = [];
      } finally {
        isLoading = false;
      }
    }, 250);
  });

  function handleKeywordClick(keyword: string) {
    saveRecentSearch(keyword);
    onSelectKeyword(keyword);
    onClose();
  }

  function handleProductClick(product: ProductSuggestionItem) {
    saveRecentSearch(product.name);
    onSelectProduct(product);
    onClose();
  }

  function handleCategoryClick(cat: string) {
    saveRecentSearch(cat);
    onSelectKeyword(cat);
    onClose();
  }

  export function handleKeyDown(e: KeyboardEvent) {
    if (!isOpen) return;

    const totalItems = productSuggestions.length + categorySuggestions.length;

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = (selectedIndex + 1) % (totalItems || 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = (selectedIndex - 1 + (totalItems || 1)) % (totalItems || 1);
    } else if (e.key === 'Enter') {
      if (selectedIndex >= 0) {
        e.preventDefault();
        if (selectedIndex < productSuggestions.length) {
          handleProductClick(productSuggestions[selectedIndex]);
        } else {
          const catIdx = selectedIndex - productSuggestions.length;
          if (catIdx < categorySuggestions.length) {
            handleCategoryClick(categorySuggestions[catIdx]);
          }
        }
      } else if (query.trim()) {
        saveRecentSearch(query.trim());
        onSelectKeyword(query.trim());
        onClose();
      }
    } else if (e.key === 'Escape') {
      onClose();
    }
  }
</script>

{#if isOpen}
  <div class="search-dropdown-card" role="listbox" tabindex="-1">
    <!-- State 1: Active query suggestions -->
    {#if query.trim().length > 0}
      {#if isLoading}
        <div class="dropdown-loading">
          <span class="spinner"></span>
          <span>Mencari saran produk...</span>
        </div>
      {:else if productSuggestions.length === 0 && categorySuggestions.length === 0}
        <div class="empty-state">
          <span class="empty-icon">🔍</span>
          <p>Tidak ditemukan saran untuk "<strong>{query}</strong>"</p>
          <button
            type="button"
            class="search-all-btn"
            onclick={() => handleKeywordClick(query.trim())}
          >
            Cari semua untuk "{query.trim()}" ↵
          </button>
        </div>
      {:else}
        <!-- Categories suggestion chips -->
        {#if categorySuggestions.length > 0}
          <div class="section-group">
            <div class="section-title">Kategori Terkait</div>
            <div class="category-chips">
              {#each categorySuggestions as cat, idx}
                <button
                  type="button"
                  class="cat-chip"
                  class:active={selectedIndex === productSuggestions.length + idx}
                  onclick={() => handleCategoryClick(cat)}
                >
                  📁 {cat}
                </button>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Product Suggestions -->
        {#if productSuggestions.length > 0}
          <div class="section-group">
            <div class="section-title">Produk Terkait</div>
            <div class="product-list">
              {#each productSuggestions as item, idx}
                <button
                  type="button"
                  class="product-item"
                  class:active={selectedIndex === idx}
                  onclick={() => handleProductClick(item)}
                >
                  <div class="thumb-wrapper">
                    {#if item.image_url}
                      <img src={item.image_url} alt={item.name} class="thumb-img" />
                    {:else}
                      <div class="thumb-placeholder">🛍️</div>
                    {/if}
                  </div>
                  <div class="product-info">
                    <span class="product-name">{item.name}</span>
                    <div class="product-meta">
                      <span class="product-category">{item.category}</span>
                      <span class="product-price">{formatRupiah(item.price_cents)}</span>
                    </div>
                  </div>
                  <span class="action-arrow">→</span>
                </button>
              {/each}
            </div>
          </div>
        {/if}

        <button
          type="button"
          class="view-all-results-btn"
          onclick={() => handleKeywordClick(query.trim())}
        >
          Lihat semua hasil untuk "<strong>{query.trim()}</strong>" →
        </button>
      {/if}

    <!-- State 2: Empty query - Show Recent Searches & Popular Searches -->
    {:else}
      {#if recentSearches.length > 0}
        <div class="section-group">
          <div class="section-header-flex">
            <span class="section-title">🕒 Riwayat Pencarian</span>
            <button
              type="button"
              class="clear-all-btn"
              onclick={clearAllRecentSearches}
            >
              Hapus Semua
            </button>
          </div>
          <div class="recent-tags">
            {#each recentSearches as keyword}
              <div class="recent-tag">
                <button
                  type="button"
                  class="tag-text-btn"
                  onclick={() => handleKeywordClick(keyword)}
                >
                  {keyword}
                </button>
                <button
                  type="button"
                  class="tag-remove-btn"
                  aria-label="Hapus dari riwayat"
                  onclick={(e) => removeRecentSearch(keyword, e)}
                >
                  ✕
                </button>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      {#if popularKeywords.length > 0}
        <div class="section-group">
          <div class="section-title">🔥 Pencarian Populer</div>
          <div class="popular-chips">
            {#each popularKeywords as item}
              <button
                type="button"
                class="popular-chip"
                onclick={() => handleKeywordClick(item.keyword)}
              >
                <span class="popular-sparkle">✨</span>
                <span class="popular-text">{item.keyword}</span>
                {#if item.search_count > 1}
                  <span class="popular-count">({item.search_count})</span>
                {/if}
              </button>
            {/each}
          </div>
        </div>
      {/if}
    {/if}
  </div>
{/if}

<style>
  .search-dropdown-card {
    position: absolute;
    top: calc(100% + 8px);
    left: 0;
    right: 0;
    background: #181c24;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 14px;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(255, 255, 255, 0.05);
    backdrop-filter: blur(12px);
    z-index: 1000;
    overflow: hidden;
    max-height: 480px;
    overflow-y: auto;
    animation: fadeInSlide 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes fadeInSlide {
    from {
      opacity: 0;
      transform: translateY(-6px) scale(0.99);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .section-group {
    padding: 12px 14px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }

  .section-header-flex {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }

  .section-title {
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #94a3b8;
    margin-bottom: 8px;
  }

  .clear-all-btn {
    background: none;
    border: none;
    color: #f43f5e;
    font-size: 0.75rem;
    font-weight: 600;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
    transition: background 0.15s;
  }

  .clear-all-btn:hover {
    background: rgba(244, 63, 94, 0.12);
  }

  .recent-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .recent-tag {
    display: inline-flex;
    align-items: center;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 20px;
    padding: 2px 4px 2px 10px;
    font-size: 0.8rem;
    color: #e2e8f0;
    transition: all 0.15s ease;
  }

  .recent-tag:hover {
    background: rgba(255, 255, 255, 0.12);
    border-color: rgba(255, 255, 255, 0.2);
  }

  .tag-text-btn {
    background: none;
    border: none;
    color: inherit;
    font-size: inherit;
    cursor: pointer;
    padding: 2px 4px 2px 0;
  }

  .tag-remove-btn {
    background: none;
    border: none;
    color: #94a3b8;
    cursor: pointer;
    font-size: 0.7rem;
    padding: 2px 6px;
    border-radius: 50%;
    line-height: 1;
    transition: color 0.15s, background 0.15s;
  }

  .tag-remove-btn:hover {
    color: #f43f5e;
    background: rgba(244, 63, 94, 0.15);
  }

  .popular-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .popular-chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    background: rgba(234, 179, 8, 0.1);
    border: 1px solid rgba(234, 179, 8, 0.25);
    border-radius: 20px;
    padding: 5px 11px;
    font-size: 0.8rem;
    color: #fef08a;
    cursor: pointer;
    transition: all 0.18s ease;
  }

  .popular-chip:hover {
    background: rgba(234, 179, 8, 0.2);
    border-color: rgba(234, 179, 8, 0.45);
    transform: translateY(-1px);
  }

  .popular-count {
    font-size: 0.7rem;
    color: rgba(254, 240, 138, 0.7);
  }

  .category-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .cat-chip {
    background: rgba(59, 130, 246, 0.12);
    border: 1px solid rgba(59, 130, 246, 0.3);
    color: #93c5fd;
    padding: 4px 10px;
    border-radius: 8px;
    font-size: 0.8rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .cat-chip:hover,
  .cat-chip.active {
    background: rgba(59, 130, 246, 0.25);
    border-color: #3b82f6;
    color: #ffffff;
  }

  .product-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .product-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 10px;
    border-radius: 8px;
    background: transparent;
    border: none;
    width: 100%;
    text-align: left;
    cursor: pointer;
    transition: background 0.12s ease;
  }

  .product-item:hover,
  .product-item.active {
    background: rgba(255, 255, 255, 0.08);
  }

  .thumb-wrapper {
    width: 42px;
    height: 42px;
    border-radius: 6px;
    overflow: hidden;
    background: #232936;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .thumb-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .thumb-placeholder {
    font-size: 1.2rem;
  }

  .product-info {
    flex: 1;
    min-width: 0;
  }

  .product-name {
    display: block;
    font-size: 0.85rem;
    font-weight: 600;
    color: #f1f5f9;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .product-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 2px;
  }

  .product-category {
    font-size: 0.72rem;
    color: #94a3b8;
    background: rgba(255, 255, 255, 0.06);
    padding: 1px 6px;
    border-radius: 4px;
  }

  .product-price {
    font-size: 0.8rem;
    font-weight: 700;
    color: #34d399;
  }

  .action-arrow {
    color: #64748b;
    font-size: 0.9rem;
    transition: transform 0.12s;
  }

  .product-item:hover .action-arrow,
  .product-item.active .action-arrow {
    color: #38bdf8;
    transform: translateX(2px);
  }

  .dropdown-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 24px;
    color: #94a3b8;
    font-size: 0.85rem;
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid rgba(255, 255, 255, 0.2);
    border-top-color: #38bdf8;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .empty-state {
    padding: 24px 16px;
    text-align: center;
    color: #94a3b8;
  }

  .empty-icon {
    font-size: 1.8rem;
    display: block;
    margin-bottom: 6px;
  }

  .search-all-btn {
    margin-top: 10px;
    background: rgba(56, 189, 248, 0.12);
    border: 1px solid rgba(56, 189, 248, 0.3);
    color: #38bdf8;
    padding: 6px 14px;
    border-radius: 8px;
    font-size: 0.8rem;
    cursor: pointer;
    transition: all 0.15s;
  }

  .search-all-btn:hover {
    background: rgba(56, 189, 248, 0.22);
    border-color: #38bdf8;
  }

  .view-all-results-btn {
    display: block;
    width: 100%;
    padding: 10px;
    background: rgba(255, 255, 255, 0.03);
    border: none;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    color: #38bdf8;
    font-size: 0.8rem;
    text-align: center;
    cursor: pointer;
    transition: background 0.15s;
  }

  .view-all-results-btn:hover {
    background: rgba(56, 189, 248, 0.1);
  }
</style>
