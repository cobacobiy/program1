# Issue 13: Migrasi Halaman Katalog Produk, Search & Dynamic Filtering

> **Prioritas:** 🟡 HIGH  
> **Estimasi:** 3-4 hari  
> **Kesulitan:** ⭐⭐⭐ Menengah  
> **Prerequisite:** Issue 11 & Issue 12

---

## 📋 Deskripsi

Di implementasi lama (`store-catalog.js` dan `store.html`), render kartu produk dilakukan menggunakan template string JavaScript murni (`innerHTML = ...`). Hal ini rentan terhadap kesalahan rendering, sulit dikelola, dan tidak memiliki animasi transisi yang mulus.  

Di Issue ini, kita akan:
1. Membangun antarmuka katalog produk berbasis komponen deklaratif Svelte 5.
2. Mengintegrasikan pencarian produk (dengan debounce), filter kategori/harga, dan pengurutan (sorting).
3. Menambahkan pagination interaktif yang memanggil endpoint backend Rust Axum `GET /catalog`.
4. Menambahkan modal detail produk dengan visualisasi stok dan tombol tambah keranjang.

---

## 🎯 Acceptance Criteria

- [ ] Data katalog diambil secara dinamis dari endpoint `GET /catalog` dengan query parameter `page`, `page_size`, `search`, `sort_by`, dan `order`.
- [ ] Komponen `ProductCard.svelte` menampilkan gambar (mendukung path `/uploads/...` maupun SVG placeholder), nama, harga Rupiah, dan status stok.
- [ ] Search bar memiliki mekanisme debounce (~300ms) agar tidak membanjiri server dengan request HTTP pada setiap ketukan keyboard.
- [ ] Tersedia efek loading skeleton saat data produk sedang di-fetch.
- [ ] Komponen `Pagination.svelte` memungkinkan navigasi antar halaman produk.
- [ ] Klik kartu produk membuka `ProductDetailModal.svelte` dengan deskripsi lengkap.

---

## 📐 Langkah-Langkah Pengerjaan

### Langkah 1: Definisikan Tipe Data Katalog

**File:** `frontend/src/lib/types/catalog.ts`

```typescript
export interface Product {
  id: string;
  name: string;
  description: string;
  price_cents: number; // Dalam satuan sen/rupiah
  stock: number;
  image_url?: string | null;
  category?: string | null;
  created_at?: string;
}

export interface CatalogPageResponse {
  items: Product[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

export interface CatalogFilters {
  search: string;
  page: number;
  pageSize: number;
  sortBy: string;
  sortOrder: 'asc' | 'desc';
}
```

---

### Langkah 2: Helper Format Rupiah & API Service

**File:** `frontend/src/lib/utils/currency.ts`

```typescript
export function formatRupiah(cents: number): string {
  // Jika backend menyimpan dalam rupiah langsung atau sen, sesuaikan nilainya
  return new Intl.NumberFormat('id-ID', {
    style: 'currency',
    currency: 'IDR',
    maximumFractionDigits: 0,
  }).format(cents);
}
```

**File:** `frontend/src/lib/api/catalog.ts`

```typescript
import { apiFetch } from './client';
import type { CatalogFilters, CatalogPageResponse } from '../types/catalog';

export async function getCatalog(filters: CatalogFilters): Promise<CatalogPageResponse> {
  const params = new URLSearchParams({
    page: filters.page.toString(),
    page_size: filters.pageSize.toString(),
  });

  if (filters.search.trim()) {
    params.set('search', filters.search.trim());
  }
  if (filters.sortBy) {
    params.set('sort_by', filters.sortBy);
    params.set('order', filters.sortOrder);
  }

  return apiFetch<CatalogPageResponse>(`/catalog?${params.toString()}`);
}
```

---

### Langkah 3: Buat Komponen `ProductCard.svelte`

**File:** `frontend/src/lib/components/ProductCard.svelte`

```svelte
<script lang="ts">
  import type { Product } from '$lib/types/catalog';
  import { formatRupiah } from '$lib/utils/currency';

  interface Props {
    product: Product;
    onSelect: (product: Product) => void;
    onAddToCart: (product: Product) => void;
  }

  let { product, onSelect, onAddToCart }: Props = $props();

  const isOutOfStock = $derived(product.stock <= 0);
</script>

<div class="product-card" onclick={() => onSelect(product)} role="button" tabindex="0">
  <div class="image-wrapper">
    {#if product.image_url}
      <img src={product.image_url} alt={product.name} loading="lazy" />
    {:else}
      <div class="placeholder-img">📦</div>
    {/if}
    {#if isOutOfStock}
      <span class="badge out-of-stock">Habis</span>
    {/if}
  </div>

  <div class="product-info">
    <h3 class="title">{product.name}</h3>
    <p class="price">{formatRupiah(product.price_cents)}</p>
    <div class="stock-info">
      <small>Stok: {product.stock} unit</small>
    </div>

    <button
      class="btn-add-cart"
      disabled={isOutOfStock}
      onclick={(e) => {
        e.stopPropagation();
        onAddToCart(product);
      }}
    >
      {isOutOfStock ? 'Stok Habis' : '+ Keranjang'}
    </button>
  </div>
</div>

<style>
  .product-card {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 10px;
    overflow: hidden;
    transition: transform 0.2s, box-shadow 0.2s;
    cursor: pointer;
    display: flex;
    flex-direction: column;
  }
  .product-card:hover {
    transform: translateY(-4px);
    box-shadow: 0 8px 20px rgba(0,0,0,0.3);
    border-color: #0284c7;
  }
  .image-wrapper {
    position: relative;
    width: 100%;
    height: 180px;
    background: #0f172a;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .image-wrapper img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .placeholder-img { font-size: 3rem; }
  .badge.out-of-stock {
    position: absolute;
    top: 8px;
    right: 8px;
    background: #dc2626;
    color: white;
    font-size: 0.75rem;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    font-weight: bold;
  }
  .product-info { padding: 1rem; flex: 1; display: flex; flex-direction: column; }
  .title {
    font-size: 1rem;
    margin: 0 0 0.5rem;
    color: #f8fafc;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .price { font-size: 1.15rem; font-weight: bold; color: #38bdf8; margin: 0 0 0.5rem; }
  .stock-info { margin-bottom: 1rem; color: #94a3b8; font-size: 0.85rem; }
  .btn-add-cart {
    margin-top: auto;
    background: #0284c7;
    color: white;
    border: none;
    padding: 0.6rem;
    border-radius: 6px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.2s;
  }
  .btn-add-cart:hover:not(:disabled) { background: #0369a1; }
  .btn-add-cart:disabled { background: #475569; cursor: not-allowed; }
</style>
```

---

### Langkah 4: Buat Komponen Search & Filter Bar

**File:** `frontend/src/lib/components/FilterBar.svelte`

```svelte
<script lang="ts">
  interface Props {
    search: string;
    sortBy: string;
    onFilterChange: (search: string, sortBy: string) => void;
  }

  let { search, sortBy, onFilterChange }: Props = $props();

  let searchInput = $state(search);
  let selectedSort = $state(sortBy);
  let debounceTimer: ReturnType<typeof setTimeout>;

  function handleInput(e: Event) {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      onFilterChange(searchInput, selectedSort);
    }, 300);
  }

  function handleSortChange() {
    onFilterChange(searchInput, selectedSort);
  }
</script>

<div class="filter-bar">
  <div class="search-box">
    <input
      type="text"
      placeholder="Cari produk impianmu..."
      bind:value={searchInput}
      oninput={handleInput}
    />
  </div>

  <div class="sort-box">
    <label for="sort-select">Urutkan:</label>
    <select id="sort-select" bind:value={selectedSort} onchange={handleSortChange}>
      <option value="created_at_desc">Terbaru</option>
      <option value="price_asc">Harga Termurah</option>
      <option value="price_desc">Harga Termahal</option>
      <option value="name_asc">Nama (A - Z)</option>
    </select>
  </div>
</div>

<style>
  .filter-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1.5rem;
  }
  .search-box { flex: 1; min-width: 250px; }
  .search-box input {
    width: 100%;
    padding: 0.75rem 1rem;
    border-radius: 8px;
    border: 1px solid #334155;
    background: #1e293b;
    color: #fff;
    font-size: 0.95rem;
  }
  .sort-box { display: flex; align-items: center; gap: 0.5rem; color: #94a3b8; }
  .sort-box select {
    background: #1e293b;
    color: #f8fafc;
    border: 1px solid #334155;
    padding: 0.6rem 0.8rem;
    border-radius: 6px;
    cursor: pointer;
  }
</style>
```

---

### Langkah 5: Buat Komponen Grid Produk & Skeleton

**File:** `frontend/src/lib/components/ProductGrid.svelte`

```svelte
<script lang="ts">
  import type { Product } from '$lib/types/catalog';
  import ProductCard from './ProductCard.svelte';

  interface Props {
    products: Product[];
    loading: boolean;
    onSelect: (p: Product) => void;
    onAddToCart: (p: Product) => void;
  }

  let { products, loading, onSelect, onAddToCart }: Props = $props();
</script>

{#if loading}
  <div class="grid">
    {#each Array(8) as _}
      <div class="skeleton-card">
        <div class="skeleton-img"></div>
        <div class="skeleton-text title"></div>
        <div class="skeleton-text price"></div>
      </div>
    {/each}
  </div>
{:else if products.length === 0}
  <div class="empty-state">
    <p>🔍 Tidak ada produk yang sesuai dengan pencarian Anda.</p>
  </div>
{:else}
  <div class="grid">
    {#each products as product (product.id)}
      <ProductCard {product} {onSelect} {onAddToCart} />
    {/each}
  </div>
{/if}

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 1.5rem;
  }
  .skeleton-card {
    background: #1e293b;
    border-radius: 10px;
    height: 300px;
    padding: 1rem;
    animation: pulse 1.5s infinite;
  }
  .skeleton-img { height: 160px; background: #334155; border-radius: 6px; margin-bottom: 1rem; }
  .skeleton-text { height: 16px; background: #334155; border-radius: 4px; margin-bottom: 0.5rem; }
  .skeleton-text.title { width: 80%; }
  .skeleton-text.price { width: 50%; }
  .empty-state {
    text-align: center;
    padding: 3rem;
    color: #94a3b8;
    background: #1e293b;
    border-radius: 10px;
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }
</style>
```

---

## 🧪 Pengujian & Verifikasi

1. Jalankan frontend: `bun run dev` dan buka `http://localhost:5173`.
2. Pastikan daftar produk tampil rapi dalam tata letak grid kartu.
3. Ketik kata kunci di kolom pencarian (misal: "kopi" atau "sepatu"), pastikan request ter-filter tanpa perlu menekan tombol submit.
4. Ubah dropdown urutan ke "Harga Termurah", pastikan urutan produk berubah secara instan.
5. Klik kartu produk untuk memastikan event seleksi berfungsi dengan baik.
6. Jalankan tes Rust: `cargo test --workspace` untuk memverifikasi backend.

---

## ⚠️ Hal Penting & Gotchas

- **Debounce:** Selalu bersihkan timer (`clearTimeout`) sebelum membuat timer baru di input pencarian untuk mencegah memory leak.
- **Image URL:** Backend menyajikan gambar yang di-upload melalui folder `/uploads/...`. Jangan mengubah domain gambar jika berformat path relatif.
