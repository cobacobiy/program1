<script lang="ts">
  import type { Category, ProductSuggestionItem } from '../types';
  import { i18n } from '../i18n.svelte';
  import SearchDropdown from '../SearchDropdown.svelte';

  interface Props {
    searchQuery: string;
    selectedCategory: string;
    categoriesList: Category[];
    totalProductCount: number;
    onUpdateQuery: (query: string) => void;
    onSelectCategory: (category: string) => void;
    onSelectProduct: (item: ProductSuggestionItem) => void;
    onSelectKeyword: (keyword: string) => void;
  }

  let {
    searchQuery,
    selectedCategory,
    categoriesList,
    totalProductCount,
    onUpdateQuery,
    onSelectCategory,
    onSelectProduct,
    onSelectKeyword,
  }: Props = $props();

  let isSearchDropdownOpen = $state(false);
  let searchDropdownRef: any = $state(null);
</script>

<div class="catalog-filters">
  <div class="search-field-wrapper">
    <div class="search-field">
      <span class="s-icon">🔍</span>
      <input
        type="text"
        placeholder={i18n.t('catalog.search_placeholder', 'Cari produk impianmu...')}
        value={searchQuery}
        oninput={(e) => onUpdateQuery((e.target as HTMLInputElement).value)}
        onfocus={() => isSearchDropdownOpen = true}
        onkeydown={(e) => searchDropdownRef?.handleKeyDown(e)}
      />
      {#if searchQuery}
        <button
          type="button"
          class="clear-query-btn"
          onclick={() => onUpdateQuery('')}
          aria-label="Bersihkan pencarian"
        >
          ✕
        </button>
      {/if}
    </div>

    <SearchDropdown
      bind:this={searchDropdownRef}
      query={searchQuery}
      bind:isOpen={isSearchDropdownOpen}
      onSelectProduct={onSelectProduct}
      onSelectKeyword={onSelectKeyword}
      onClose={() => isSearchDropdownOpen = false}
    />
  </div>

  {#if categoriesList.length > 0}
    <div class="category-pills" role="tablist" aria-label="Filter kategori">
      <button
        class="pill-btn"
        class:active={selectedCategory === 'all'}
        onclick={() => onSelectCategory('all')}
        type="button"
      >
        <span class="pill-icon">🏷️</span>
        <span class="pill-label">{i18n.t('catalog.all_categories', 'Semua Kategori')}</span>
        {#if totalProductCount > 0}
          <span class="pill-count">{totalProductCount}</span>
        {/if}
      </button>
      {#each categoriesList as cat (cat.id)}
        <button
          class="pill-btn"
          class:active={selectedCategory === cat.slug || selectedCategory === cat.name || selectedCategory === cat.id}
          onclick={() => onSelectCategory(cat.name)}
          type="button"
        >
          {#if cat.icon}
            <span class="pill-icon">{cat.icon}</span>
          {/if}
          <span class="pill-label">{cat.name}</span>
          {#if cat.product_count > 0}
            <span class="pill-count">{cat.product_count}</span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .catalog-filters { display: flex; flex-direction: column; gap: 1rem; margin-bottom: 1.75rem; }
  .search-field-wrapper {
    position: relative;
    width: 100%;
  }
  .search-field {
    display: flex; align-items: center; background: #0f172a; border: 1px solid #334155;
    border-radius: 8px; padding: 0 0.85rem;
  }
  .s-icon { margin-right: 0.5rem; font-size: 1rem; }
  .search-field input {
    width: 100%; background: transparent; border: none; padding: 0.75rem 0;
    color: #fff; font-size: 0.95rem;
  }
  .search-field input:focus { outline: none; }
  .clear-query-btn {
    background: none;
    border: none;
    color: #94a3b8;
    cursor: pointer;
    font-size: 0.8rem;
    padding: 2px 6px;
    border-radius: 50%;
    line-height: 1;
    transition: color 0.15s, background 0.15s;
  }
  .clear-query-btn:hover {
    color: #f43f5e;
    background: rgba(244, 63, 94, 0.15);
  }
  .category-pills { display: flex; gap: 0.5rem; flex-wrap: wrap; }
  .pill-btn {
    background: #1e293b; color: #94a3b8; border: 1px solid #334155;
    padding: 0.4rem 0.85rem; border-radius: 20px; font-size: 0.85rem; cursor: pointer;
    display: inline-flex; align-items: center; gap: 0.4rem; transition: all 0.15s ease;
  }
  .pill-btn:hover { background: #334155; color: #f1f5f9; border-color: #475569; }
  .pill-btn.active {
    background: #0284c7; color: #fff; border-color: #0284c7; font-weight: 600;
    box-shadow: 0 2px 8px rgba(2, 132, 199, 0.4);
  }
  .pill-icon { font-size: 1rem; line-height: 1; }
  .pill-label { line-height: 1; }
  .pill-count {
    background: rgba(255, 255, 255, 0.15); color: #cbd5e1; font-size: 0.72rem; font-weight: 700;
    padding: 0.1rem 0.45rem; border-radius: 9999px; line-height: 1;
  }
  .pill-btn.active .pill-count {
    background: rgba(255, 255, 255, 0.25); color: #ffffff;
  }
</style>
