<script lang="ts">
  import { onMount } from 'svelte';
  import type {
    HealthResponse,
    Product,
    CatalogPageResponse,
    ProductVariant,
    PublicReview,
    ProductRatingSummary,
    PaginatedPublicReviews,
    WishlistItem,
    Category,
  } from './lib/types';
  import { formatRupiah } from './lib/currency';
  import { auth } from './lib/auth.svelte';
  import { cart } from './lib/cart.svelte';
  import { toast } from './lib/toast.svelte';
  import { i18n } from './lib/i18n.svelte';
  import { apiFetch } from './lib/api';

  import AuthModal from './lib/AuthModal.svelte';
  import CheckoutModal from './lib/CheckoutModal.svelte';
  import BuyerOrdersModal from './lib/BuyerOrdersModal.svelte';
  import WishlistModal from './lib/WishlistModal.svelte';
  import LiveChat from './lib/LiveChat.svelte';
  import AdminHub from './lib/AdminHub.svelte';

  // Navigation & Modals state
  let activeTab = $state<'store' | 'admin' | 'diagnostic'>('store');
  let isCheckoutOpen = $state(false);
  let isOrdersOpen = $state(false);
  let isWishlistOpen = $state(false);
  let wishlist = $state<WishlistItem[]>([]);
  let wishlistLoading = $state(false);

  function isProductInWishlist(productId: string): boolean {
    return wishlist.some((item) => item.product_id === productId);
  }

  async function loadWishlist() {
    if (!auth.user) {
      wishlist = [];
      return;
    }
    wishlistLoading = true;
    try {
      wishlist = await apiFetch<WishlistItem[]>('/api/v1/buyer/wishlist');
    } catch {
      wishlist = [];
    } finally {
      wishlistLoading = false;
    }
  }

  async function toggleWishlist(product: Product) {
    if (!auth.user) {
      toast.info('Silakan masuk terlebih dahulu untuk menambahkan ke Wishlist.');
      auth.isModalOpen = true;
      return;
    }

    if (isProductInWishlist(product.id)) {
      try {
        await apiFetch(`/api/v1/buyer/wishlist/${product.id}`, { method: 'DELETE' });
        wishlist = wishlist.filter((item) => item.product_id !== product.id);
        toast.info(`"${product.name}" dihapus dari wishlist.`);
      } catch (err: any) {
        toast.error(err.message || 'Gagal menghapus dari wishlist.');
      }
    } else {
      try {
        const item = await apiFetch<WishlistItem>(`/api/v1/buyer/wishlist/${product.id}`, {
          method: 'POST',
        });
        wishlist = [item, ...wishlist];
        toast.success(`"${product.name}" ditambahkan ke wishlist! ❤️`);
      } catch (err: any) {
        toast.error(err.message || 'Gagal menambahkan ke wishlist.');
      }
    }
  }

  async function handleRemoveWishlistItem(productId: string) {
    try {
      await apiFetch(`/api/v1/buyer/wishlist/${productId}`, { method: 'DELETE' });
      wishlist = wishlist.filter((item) => item.product_id !== productId);
      toast.info('Produk dihapus dari wishlist.');
    } catch (err: any) {
      toast.error(err.message || 'Gagal menghapus dari wishlist.');
    }
  }

  function handleAddWishlistItemToCart(item: WishlistItem) {
    const matched = products.find((p) => p.id === item.product_id);
    const prod: Product = matched || {
      id: item.product_id,
      name: item.product_name,
      description: '',
      price_cents: item.product_price_cents,
      stock: 99,
      image_url: item.product_image_url,
    };
    cart.addItem(prod);
  }

  function openWishlistModal() {
    if (!auth.user) {
      toast.info('Silakan masuk terlebih dahulu untuk melihat Wishlist kamu.');
      auth.isModalOpen = true;
      return;
    }
    isWishlistOpen = true;
    loadWishlist();
  }

  $effect(() => {
    if (auth.user) {
      loadWishlist();
    } else {
      wishlist = [];
    }
  });

  // Quick Variant Picker Modal State
  let variantPickerProduct = $state<Product | null>(null);
  let availableVariants = $state<ProductVariant[]>([]);
  let selectedVariant = $state<ProductVariant | null>(null);
  let variantLoading = $state(false);

  // Review & Rating State
  let ratingSummaries = $state<Record<string, ProductRatingSummary>>({});
  let selectedProductForDetail = $state<Product | null>(null);
  let activeRatingSummary = $state<ProductRatingSummary | null>(null);
  let activeReviews = $state<PublicReview[]>([]);
  let reviewsLoading = $state(false);
  let reviewPage = $state(1);
  let reviewTotalPages = $state(1);
  let reviewTotal = $state(0);

  async function handleAddToCartClick(product: Product) {
    variantLoading = true;
    try {
      const vars = await apiFetch<ProductVariant[]>(`/catalog/${product.id}/variants`);
      if (vars && vars.length > 0) {
        variantPickerProduct = product;
        availableVariants = vars;
        selectedVariant = vars[0];
        return;
      }
    } catch {
      // Fallback to base product if fetch fails or no variants exist
    } finally {
      variantLoading = false;
    }
    cart.addItem(product);
  }

  function confirmAddVariantToCart() {
    if (!variantPickerProduct) return;
    cart.addItem(variantPickerProduct, 1, selectedVariant || undefined);
    variantPickerProduct = null;
  }

  // Health state
  let healthData = $state<HealthResponse | null>(null);
  let healthLoading = $state(false);
  let healthError = $state<string | null>(null);

  // Catalog & Category state
  let products = $state<Product[]>([]);
  let categoriesList = $state<Category[]>([]);
  let categoriesLoading = $state(false);
  let catalogLoading = $state(false);
  let catalogError = $state<string | null>(null);
  let searchQuery = $state('');
  let selectedCategory = $state<string>('all');

  const totalProductCount = $derived(
    categoriesList.length > 0
      ? categoriesList.reduce((acc, c) => acc + (c.product_count || 0), 0)
      : products.length
  );

  const filteredProducts = $derived(
    products.filter(p => {
      const matchSearch = p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
                          p.description?.toLowerCase().includes(searchQuery.toLowerCase());
      const matchCategory = selectedCategory === 'all' ||
                            p.category === selectedCategory ||
                            p.category_id === selectedCategory ||
                            (p.category_name && p.category_name.toLowerCase() === selectedCategory.toLowerCase()) ||
                            (categoriesList.find(c => c.slug === selectedCategory)?.name === p.category);
      return matchSearch && matchCategory;
    })
  );

  async function fetchCategories() {
    categoriesLoading = true;
    try {
      categoriesList = await apiFetch<Category[]>('/api/v1/categories');
    } catch {
      categoriesList = [];
    } finally {
      categoriesLoading = false;
    }
  }

  async function checkHealth() {
    healthLoading = true;
    healthError = null;
    try {
      healthData = await apiFetch<HealthResponse>('/health');
    } catch (err: any) {
      healthError = err.message || 'Gagal terhubung ke server backend';
    } finally {
      healthLoading = false;
    }
  }

  async function fetchCatalog() {
    catalogLoading = true;
    catalogError = null;
    try {
      const res = await apiFetch<CatalogPageResponse>('/catalog?page=1&page_size=30');
      products = res.items || [];
      // Fetch rating summaries for all products in parallel
      for (const p of products) {
        apiFetch<ProductRatingSummary>(`/api/v1/catalog/${p.id}/rating`)
          .then((summary) => {
            ratingSummaries[p.id] = summary;
          })
          .catch(() => {});
      }
    } catch (err: any) {
      catalogError = err.message || 'Gagal memuat katalog produk';
    } finally {
      catalogLoading = false;
    }
  }

  async function openProductDetail(product: Product) {
    selectedProductForDetail = product;
    reviewPage = 1;
    activeRatingSummary = ratingSummaries[product.id] || null;
    await Promise.all([
      fetchRatingSummary(product.id),
      fetchProductReviews(product.id, 1),
    ]);
  }

  async function fetchRatingSummary(productId: string) {
    try {
      const sum = await apiFetch<ProductRatingSummary>(`/api/v1/catalog/${productId}/rating`);
      activeRatingSummary = sum;
      ratingSummaries[productId] = sum;
    } catch {
      // ignore
    }
  }

  async function fetchProductReviews(productId: string, page: number) {
    reviewsLoading = true;
    try {
      const res = await apiFetch<PaginatedPublicReviews>(`/api/v1/catalog/${productId}/reviews?page=${page}&page_size=5`);
      activeReviews = res.data || [];
      reviewPage = res.page;
      reviewTotalPages = res.total_pages;
      reviewTotal = res.total;
    } catch {
      activeReviews = [];
    } finally {
      reviewsLoading = false;
    }
  }

  function changeReviewPage(newPage: number) {
    if (!selectedProductForDetail) return;
    if (newPage < 1 || newPage > reviewTotalPages) return;
    fetchProductReviews(selectedProductForDetail.id, newPage);
  }

  function handleStartCheckout() {
    cart.isOpen = false;
    if (!auth.user) {
      toast.info('Silakan masuk terlebih dahulu untuk melanjutkan pembayaran.');
      auth.isModalOpen = true;
      return;
    }
    isCheckoutOpen = true;
  }

  onMount(() => {
    checkHealth();
    fetchCatalog();
    fetchCategories();
  });
</script>

<div class="app-layout">
  <!-- Toast Notification System -->
  <div class="toast-stack">
    {#each toast.toasts as t (t.id)}
      <div class="toast-card {t.type}">
        <span>{t.message}</span>
        <button class="t-close" onclick={() => toast.remove(t.id)}>&times;</button>
      </div>
    {/each}
  </div>

  <!-- Global Modals -->
  <AuthModal />
  <CheckoutModal isOpen={isCheckoutOpen} onClose={() => isCheckoutOpen = false} />
  <BuyerOrdersModal isOpen={isOrdersOpen} onClose={() => isOrdersOpen = false} />
  <WishlistModal
    isOpen={isWishlistOpen}
    onClose={() => isWishlistOpen = false}
    wishlist={wishlist}
    loading={wishlistLoading}
    onRemoveItem={handleRemoveWishlistItem}
    onAddToCart={handleAddWishlistItemToCart}
  />

  <!-- Navbar -->
  <header class="navbar">
    <div class="nav-inner">
      <div class="brand">
        <span class="logo-emoji">⚡</span>
        <div>
          <h2>Program1</h2>
          <span class="subtext">Svelte 5 + Rust Axum</span>
        </div>
      </div>

      <nav class="nav-tabs">
        <button class:active={activeTab === 'store'} onclick={() => activeTab = 'store'}>
          🏪 {i18n.t('nav.products', 'Katalog Toko')}
        </button>
        <button class:active={activeTab === 'admin'} onclick={() => activeTab = 'admin'}>
          ⚙️ {i18n.t('nav.admin', 'Admin Hub')}
        </button>
        <button class:active={activeTab === 'diagnostic'} onclick={() => activeTab = 'diagnostic'}>
          🔍 {i18n.t('nav.diagnostics', 'Server Health')}
          {#if healthData}
            <span class="status-dot green"></span>
          {:else}
            <span class="status-dot yellow"></span>
          {/if}
        </button>
      </nav>

      <div class="nav-user-actions">
        <!-- Language Switcher -->
        <div class="lang-switch-pills" role="group" aria-label={i18n.t('common.language', 'Pilih Bahasa')}>
          <button
            class="lang-pill"
            class:active={i18n.current === 'id'}
            onclick={() => i18n.setLanguage('id')}
            title="Bahasa Indonesia"
            type="button"
          >
            🇮🇩 ID
          </button>
          <button
            class="lang-pill"
            class:active={i18n.current === 'en'}
            onclick={() => i18n.setLanguage('en')}
            title="English"
            type="button"
          >
            🇬🇧 EN
          </button>
        </div>

        <button class="btn-wishlist" onclick={openWishlistModal} title={i18n.t('nav.wishlist', 'Wishlist Saya')}>
          ❤️ {i18n.t('nav.wishlist', 'Wishlist')}
          {#if wishlist.length > 0}
            <span class="wishlist-count">{wishlist.length}</span>
          {/if}
        </button>

        {#if auth.user}
          <button class="btn-orders" onclick={() => isOrdersOpen = true}>
            📦 {i18n.t('nav.orders', 'Pesanan Saya')}
          </button>
          <div class="user-pill">
            <span class="uname">👤 {auth.user.name}</span>
            <button class="btn-logout" onclick={() => auth.logout()}>{i18n.t('nav.logout', 'Keluar')}</button>
          </div>
        {:else}
          <button class="btn-login" onclick={() => auth.isModalOpen = true}>
            {i18n.t('auth.login_title', 'Masuk')} / {i18n.t('auth.register_title', 'Daftar')}
          </button>
        {/if}

        <button class="cart-trigger" onclick={() => cart.isOpen = true}>
          🛒 {i18n.t('nav.cart', 'Keranjang')}
          {#if cart.totalItems > 0}
            <span class="cart-count">{cart.totalItems}</span>
          {/if}
        </button>
      </div>
    </div>
  </header>

  <!-- Body Content -->
  <main class="page-body">
    {#if activeTab === 'store'}
      <section class="store-view">
        <div class="catalog-filters">
          <div class="search-field">
            <span class="s-icon">🔍</span>
            <input
              type="text"
              placeholder={i18n.t('catalog.search_placeholder', 'Cari produk impianmu...')}
              bind:value={searchQuery}
            />
          </div>

          {#if categoriesList.length > 0}
            <div class="category-pills" role="tablist" aria-label="Filter kategori">
              <button
                class="pill-btn"
                class:active={selectedCategory === 'all'}
                onclick={() => selectedCategory = 'all'}
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
                  onclick={() => selectedCategory = cat.name}
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

        {#if catalogLoading}
          <div class="grid-skeleton">
            {#each Array(6) as _}
              <div class="card-skeleton"></div>
            {/each}
          </div>
        {:else if catalogError}
          <div class="alert-box error">
            <p><strong>{i18n.t('catalog.load_failed', 'Gagal Memuat Produk')}:</strong> {catalogError}</p>
            <button class="btn-retry" onclick={fetchCatalog}>{i18n.t('catalog.retry', 'Coba Lagi')}</button>
          </div>
        {:else if filteredProducts.length === 0}
          <div class="empty-box">
            <p>{i18n.t('catalog.empty_search', 'Produk tidak ditemukan untuk pencarian')} "{searchQuery}".</p>
          </div>
        {:else}
          <div class="catalog-grid">
            {#each filteredProducts as product (product.id)}
              <div class="product-item">
                <div
                  class="img-container clickable"
                  onclick={() => openProductDetail(product)}
                  role="button"
                  tabindex="0"
                  onkeydown={(e) => e.key === 'Enter' && openProductDetail(product)}
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
                    onclick={(e) => { e.stopPropagation(); toggleWishlist(product); }}
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
                      onclick={() => openProductDetail(product)}
                    >
                      {product.name}
                    </button>
                  </h3>

                  <!-- Rating Preview -->
                  <button
                    class="rating-badge-btn"
                    onclick={() => openProductDetail(product)}
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
                      onclick={() => openProductDetail(product)}
                      type="button"
                    >
                      {i18n.t('catalog.view_detail', 'Detail & Ulasan')}
                    </button>
                    <button
                      class="btn-add"
                      disabled={product.stock <= 0}
                      onclick={() => handleAddToCartClick(product)}
                    >
                      {product.stock <= 0 ? i18n.t('catalog.out_of_stock', 'Habis') : `+ ${i18n.t('catalog.add_to_cart', 'Keranjang')}`}
                    </button>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </section>

    {:else if activeTab === 'diagnostic'}
      <section class="diagnostic-view">
        <div class="card-panel">
          <div class="panel-header">
            <h3>⚡ Status Backend Axum (<code>GET /health</code>)</h3>
            <button class="btn-refresh" onclick={checkHealth} disabled={healthLoading}>
              {healthLoading ? 'Memeriksa...' : 'Ping Server'}
            </button>
          </div>

          {#if healthLoading}
            <p class="info-text">Menghubungi endpoint backend...</p>
          {:else if healthError}
            <div class="alert-box error">
              <h4>❌ Sambungan Terputus</h4>
              <p>{healthError}</p>
            </div>
          {:else if healthData}
            <div class="alert-box success">
              <h4>✅ Server Rust Axum Berjalan Normal!</h4>
              <p><strong>Status API:</strong> <code>{healthData.status}</code></p>
              <p><strong>Versi Binary:</strong> <code>{healthData.version}</code></p>
              {#if healthData.subsystems}
                <div class="subsystems-dump">
                  <strong>Subsystem Diagnostic:</strong>
                  <pre>{JSON.stringify(healthData.subsystems, null, 2)}</pre>
                </div>
              {/if}
            </div>
          {/if}
        </div>
      </section>

    {:else if activeTab === 'admin'}
      <AdminHub />
    {/if}
  </main>

  <!-- Slide-Over Cart Drawer -->
  {#if cart.isOpen}
    <div
      class="drawer-backdrop"
      onclick={() => cart.isOpen = false}
      role="button"
      tabindex="0"
      onkeydown={(e) => e.key === 'Escape' && (cart.isOpen = false)}
    >
      <div class="cart-drawer" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
        <div class="drawer-header">
          <h3>🛒 {i18n.t('cart.title', 'Keranjang')} ({cart.totalItems})</h3>
          <button class="close-drawer" onclick={() => cart.isOpen = false}>&times;</button>
        </div>

        <div class="drawer-body">
          {#if cart.items.length === 0}
            <div class="empty-cart-state">
              <p>{i18n.t('cart.empty', 'Keranjang Anda masih kosong.')}</p>
              <button class="btn-explore" onclick={() => cart.isOpen = false}>{i18n.t('cart.continue_shopping', 'Mulai Belanja')}</button>
            </div>
          {:else}
            <div class="cart-lines">
              {#each cart.items as item (`${item.product.id}_${item.variant?.id || 'base'}`)}
                <div class="line-item">
                  <div class="line-info">
                    <h4>{item.product.name}</h4>
                    {#if item.variant}
                      <span class="variant-tag">{item.variant.variant_name}: {item.variant.variant_value}</span>
                    {/if}
                    <span class="line-price">{formatRupiah(item.variant?.price_override && item.variant.price_override > 0 ? item.variant.price_override : item.product.price_cents)}</span>
                  </div>
                  <div class="line-actions">
                    <button class="qty-btn" onclick={() => cart.updateQuantity(item.product.id, -1, item.variant?.id)}>-</button>
                    <span class="qty-num">{item.quantity}</span>
                    <button class="qty-btn" onclick={() => cart.updateQuantity(item.product.id, 1, item.variant?.id)}>+</button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>

        {#if cart.items.length > 0}
          <div class="drawer-footer">
            <div class="total-bar">
              <span>{i18n.t('cart.total', 'Total')}:</span>
              <strong class="total-text">{formatRupiah(cart.totalAmountCents)}</strong>
            </div>
            <button class="btn-checkout" onclick={handleStartCheckout}>
              {i18n.t('cart.checkout_btn', 'Lanjut ke Pembayaran')} 💳
            </button>
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- Modal Pemilihan Varian Toko -->
  {#if variantPickerProduct}
    <div class="drawer-backdrop" onclick={() => variantPickerProduct = null} role="presentation">
      <div class="modal-card variant-picker-modal" onclick={(e) => e.stopPropagation()} role="dialog">
        <div class="modal-header">
          <div>
            <h3>{i18n.t('product.select_variant', 'Pilih Varian Produk')}</h3>
            <p class="picker-prod-name">{variantPickerProduct.name}</p>
          </div>
          <button class="close-btn" onclick={() => variantPickerProduct = null}>&times;</button>
        </div>

        <div class="picker-body">
          <label class="picker-label">{i18n.t('product.select_variant', 'Pilihan Varian')} ({variantPickerProduct.name}):</label>
          <div class="variant-chips">
            {#each availableVariants as v (v.id)}
              <button
                class="v-chip"
                class:selected={selectedVariant?.id === v.id}
                onclick={() => selectedVariant = v}
              >
                <strong>{v.variant_value}</strong>
                {#if v.price_override && v.price_override > 0}
                  <small class="v-price">{formatRupiah(v.price_override)}</small>
                {/if}
                <small class="v-stock">{i18n.t('product.stock_available', 'Stok')}: {v.stock_quantity}</small>
              </button>
            {/each}
          </div>

          <div class="picker-summary">
            <div class="summary-line">
              <span>{i18n.t('product.price', 'Harga')}:</span>
              <strong class="picker-price">
                {formatRupiah(selectedVariant?.price_override && selectedVariant.price_override > 0 ? selectedVariant.price_override : variantPickerProduct.price_cents)}
              </strong>
            </div>
            <div class="summary-line">
              <span>{i18n.t('product.stock_available', 'Stok Varian')}:</span>
              <span>{selectedVariant ? selectedVariant.stock_quantity : variantPickerProduct.stock} {i18n.t('catalog.units', 'unit')}</span>
            </div>
          </div>

          <button
            class="btn-confirm-var"
            disabled={selectedVariant ? selectedVariant.stock_quantity <= 0 : false}
            onclick={confirmAddVariantToCart}
          >
            {i18n.t('product.confirm_variant', '+ Masukkan ke Keranjang')}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- Product Detail & Reviews Modal -->
  {#if selectedProductForDetail}
    <div
      class="drawer-backdrop"
      onclick={() => selectedProductForDetail = null}
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
            <h3>{selectedProductForDetail.name}</h3>
            <span class="badge-tag">{selectedProductForDetail.category || 'Umum'}</span>
          </div>
          <button class="close-btn" onclick={() => selectedProductForDetail = null}>&times;</button>
        </div>

        <div class="detail-body">
          <!-- Overview: image + details -->
          <div class="detail-overview">
            <div class="detail-img-box">
              {#if selectedProductForDetail.image_url}
                <img src={selectedProductForDetail.image_url} alt={selectedProductForDetail.name} />
              {:else}
                <span class="placeholder-emoji large">📦</span>
              {/if}
            </div>

            <div class="detail-info">
              <div class="detail-price-line">
                <span class="detail-price">{formatRupiah(selectedProductForDetail.price_cents)}</span>
                <span class="detail-stock" class:out={selectedProductForDetail.stock <= 0}>
                  {selectedProductForDetail.stock > 0 ? `${i18n.t('product.stock_available', 'Stok')}: ${selectedProductForDetail.stock} ${i18n.t('catalog.units', 'unit')}` : i18n.t('catalog.out_of_stock', 'Stok Habis')}
                </span>
              </div>
              <p class="detail-desc">{selectedProductForDetail.description || '-'}</p>

              <div class="detail-actions-row">
                <button
                  class="btn-add-detail"
                  disabled={selectedProductForDetail.stock <= 0}
                  onclick={() => {
                    if (selectedProductForDetail) {
                      handleAddToCartClick(selectedProductForDetail);
                    }
                  }}
                >
                  {selectedProductForDetail.stock <= 0 ? i18n.t('catalog.out_of_stock', 'Stok Habis') : `🛒 ${i18n.t('product.confirm_variant', 'Masukkan ke Keranjang')}`}
                </button>
                <button
                  class="btn-fav-detail"
                  class:active={isProductInWishlist(selectedProductForDetail.id)}
                  onclick={() => {
                    if (selectedProductForDetail) {
                      toggleWishlist(selectedProductForDetail);
                    }
                  }}
                  type="button"
                >
                  {isProductInWishlist(selectedProductForDetail.id) ? '❤️ Wishlist' : '🤍 Wishlist'}
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
                      onclick={() => changeReviewPage(reviewPage - 1)}
                    >
                      &larr; Sebelumnya
                    </button>
                    <span class="page-info">
                      Halaman {reviewPage} dari {reviewTotalPages} ({reviewTotal} ulasan)
                    </span>
                    <button
                      class="btn-page"
                      disabled={reviewPage >= reviewTotalPages}
                      onclick={() => changeReviewPage(reviewPage + 1)}
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

  <!-- Live Chat Widget -->
  <LiveChat />
</div>

<style>
  :global(body) {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
    background: #090d16;
    color: #f1f5f9;
  }

  .app-layout {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }

  /* Toast Stack */
  .toast-stack {
    position: fixed;
    top: 1.5rem;
    right: 1.5rem;
    z-index: 9999;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .toast-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-width: 260px;
    max-width: 380px;
    padding: 0.75rem 1rem;
    border-radius: 8px;
    font-size: 0.9rem;
    color: #fff;
    box-shadow: 0 4px 14px rgba(0,0,0,0.4);
    animation: slideRight 0.2s ease-out;
  }
  .toast-card.success { background: #059669; }
  .toast-card.error { background: #dc2626; }
  .toast-card.info { background: #0284c7; }
  .t-close { background: none; border: none; color: #fff; font-size: 1.2rem; cursor: pointer; }

  /* Navbar */
  .navbar {
    background: #0f172a;
    border-bottom: 1px solid #1e293b;
    position: sticky;
    top: 0;
    z-index: 100;
  }
  .nav-inner {
    max-width: 1200px;
    margin: 0 auto;
    padding: 0.75rem 1.5rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .brand { display: flex; align-items: center; gap: 0.6rem; }
  .logo-emoji { font-size: 1.7rem; }
  .brand h2 { margin: 0; font-size: 1.25rem; color: #38bdf8; }
  .subtext { font-size: 0.75rem; color: #94a3b8; }

  .nav-tabs { display: flex; gap: 0.5rem; }
  .nav-tabs button {
    background: transparent; border: none; color: #94a3b8;
    padding: 0.5rem 1rem; border-radius: 6px; cursor: pointer;
    font-size: 0.9rem; display: flex; align-items: center; gap: 0.4rem;
  }
  .nav-tabs button:hover { background: #1e293b; color: #fff; }
  .nav-tabs button.active { background: #1e293b; color: #38bdf8; font-weight: bold; }
  .status-dot { width: 8px; height: 8px; border-radius: 50%; }
  .status-dot.green { background: #10b981; box-shadow: 0 0 6px #10b981; }
  .status-dot.yellow { background: #f59e0b; }

  .nav-user-actions { display: flex; align-items: center; gap: 0.75rem; }
  .lang-switch-pills {
    display: flex;
    align-items: center;
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 20px;
    padding: 2px;
    gap: 2px;
  }
  .lang-pill {
    background: transparent;
    color: #94a3b8;
    border: none;
    font-size: 0.75rem;
    font-weight: 600;
    padding: 0.25rem 0.6rem;
    border-radius: 16px;
    cursor: pointer;
    transition: all 0.2s ease;
  }
  .lang-pill:hover {
    color: #fff;
  }
  .lang-pill.active {
    background: #0284c7;
    color: #fff;
    box-shadow: 0 1px 4px rgba(2, 132, 199, 0.4);
  }
  .user-pill {
    display: flex; align-items: center; gap: 0.5rem;
    background: #1e293b; padding: 0.35rem 0.75rem; border-radius: 20px; border: 1px solid #334155;
  }
  .uname { font-size: 0.85rem; color: #cbd5e1; }
  .btn-logout {
    background: #475569; color: #fff; border: none; font-size: 0.75rem;
    padding: 0.2rem 0.5rem; border-radius: 4px; cursor: pointer;
  }
  .btn-login {
    background: #0284c7; color: #fff; border: none; padding: 0.5rem 0.9rem;
    border-radius: 6px; font-size: 0.9rem; font-weight: 500; cursor: pointer;
  }
  .btn-wishlist {
    background: #1e293b; color: #f8fafc; border: 1px solid #334155;
    padding: 0.5rem 0.85rem; border-radius: 6px; font-size: 0.85rem; font-weight: 600;
    cursor: pointer; display: flex; align-items: center; gap: 0.4rem; transition: all 0.2s;
  }
  .btn-wishlist:hover {
    background: #334155; border-color: #ef4444; color: #fca5a5;
  }
  .wishlist-count {
    background: #ef4444; color: #fff; font-size: 0.7rem; font-weight: bold;
    padding: 0.1rem 0.45rem; border-radius: 9999px;
  }
  .btn-orders {
    background: #1e293b; color: #38bdf8; border: 1px solid #334155;
    padding: 0.5rem 0.8rem; border-radius: 6px; font-size: 0.85rem; cursor: pointer;
  }
  .cart-trigger {
    background: #0284c7; color: #fff; border: none; padding: 0.5rem 1rem;
    border-radius: 6px; font-weight: 600; cursor: pointer; display: flex; align-items: center; gap: 0.4rem;
  }
  .cart-count {
    background: #ef4444; color: white; font-size: 0.75rem; padding: 0.1rem 0.4rem; border-radius: 999px;
  }

  /* Page Body */
  .page-body {
    max-width: 1200px; margin: 0 auto; padding: 2rem 1.5rem; flex: 1; width: 100%; box-sizing: border-box;
  }

  /* Catalog Filters */
  .catalog-filters { display: flex; flex-direction: column; gap: 1rem; margin-bottom: 1.75rem; }
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

  /* Catalog Grid */
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
  .p-desc { font-size: 0.85rem; color: #94a3b8; margin: 0 0 1rem; flex: 1; }
  .item-meta { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
  .price-value { font-size: 1.15rem; font-weight: bold; color: #38bdf8; }
  .stock-value { color: #64748b; }
  .btn-add {
    background: #0284c7; color: #fff; border: none; padding: 0.65rem;
    border-radius: 6px; font-weight: 600; cursor: pointer; transition: background 0.2s;
  }
  .btn-add:hover:not(:disabled) { background: #0369a1; }
  .btn-add:disabled { background: #475569; cursor: not-allowed; }

  /* Diagnostic View */
  .card-panel {
    background: #1e293b; border: 1px solid #334155; border-radius: 12px; padding: 1.5rem;
  }
  .panel-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
  .panel-header h3 { margin: 0; }
  .btn-refresh {
    background: #0284c7; color: #fff; border: none; padding: 0.5rem 1rem;
    border-radius: 6px; cursor: pointer; font-weight: 500;
  }
  .alert-box { padding: 1.25rem; border-radius: 8px; margin-top: 1rem; }
  .alert-box.success { background: #064e3b; color: #a7f3d0; }
  .alert-box.error { background: #7f1d1d; color: #fecaca; }
  .subsystems-dump pre { background: #022c22; padding: 0.75rem; border-radius: 6px; overflow-x: auto; }

  /* Cart Drawer */
  .drawer-backdrop {
    position: fixed; inset: 0; background: rgba(0,0,0,0.6); z-index: 300;
    display: flex; justify-content: flex-end;
  }
  .cart-drawer {
    width: 100%; max-width: 400px; height: 100%; background: #0f172a;
    border-left: 1px solid #1e293b; display: flex; flex-direction: column;
    box-shadow: -4px 0 20px rgba(0,0,0,0.5); animation: slideLeft 0.2s ease-out;
  }
  .drawer-header {
    padding: 1.25rem; border-bottom: 1px solid #1e293b;
    display: flex; justify-content: space-between; align-items: center;
  }
  .drawer-header h3 { margin: 0; }
  .close-drawer { background: none; border: none; font-size: 1.5rem; color: #94a3b8; cursor: pointer; }
  .drawer-body { flex: 1; overflow-y: auto; padding: 1.25rem; }
  .empty-cart-state { text-align: center; color: #94a3b8; padding: 3rem 0; }
  .btn-explore {
    background: #0284c7; color: #fff; border: none; padding: 0.5rem 1rem;
    border-radius: 6px; cursor: pointer; margin-top: 1rem;
  }
  .cart-lines { display: flex; flex-direction: column; gap: 0.75rem; }
  .line-item {
    background: #1e293b; padding: 0.75rem 1rem; border-radius: 8px; border: 1px solid #334155;
    display: flex; justify-content: space-between; align-items: center;
  }
  .line-info h4 { margin: 0 0 0.25rem; font-size: 0.95rem; }
  .line-price { color: #38bdf8; font-weight: bold; font-size: 0.9rem; }
  .line-actions { display: flex; align-items: center; gap: 0.5rem; }
  .qty-btn {
    background: #334155; color: #fff; border: none; width: 26px; height: 26px;
    border-radius: 4px; cursor: pointer; font-weight: bold;
  }
  .qty-num { min-width: 20px; text-align: center; }
  .drawer-footer { padding: 1.25rem; border-top: 1px solid #1e293b; background: #0b1120; }
  .total-bar { display: flex; justify-content: space-between; font-size: 1.1rem; margin-bottom: 1rem; }
  .total-text { color: #38bdf8; }
  .btn-checkout {
    width: 100%; background: #0284c7; color: #fff; border: none;
    padding: 0.8rem; border-radius: 8px; font-size: 1rem; font-weight: bold; cursor: pointer;
  }

  @keyframes slideRight {
    from { transform: translateX(50px); opacity: 0; }
    to { transform: translateX(0); opacity: 1; }
  }
  @keyframes slideLeft {
    from { transform: translateX(100%); }
    to { transform: translateX(0); }
  }

  /* Variant Picker & Tags */
  .variant-tag {
    display: inline-block; background: #0284c7; color: #fff;
    font-size: 0.72rem; padding: 0.15rem 0.45rem; border-radius: 4px;
    margin-bottom: 0.25rem; font-weight: 500;
  }
  .variant-picker-modal {
    background: #0f172a; border: 1px solid #334155; border-radius: 12px;
    width: 90%; max-width: 440px; padding: 1.5rem; color: #f8fafc;
    box-shadow: 0 10px 30px rgba(0,0,0,0.6);
  }
  .picker-prod-name { font-size: 0.85rem; color: #38bdf8; margin: 0.25rem 0 0; }
  .picker-body { margin-top: 1rem; display: flex; flex-direction: column; gap: 1rem; }
  .picker-label { font-size: 0.85rem; color: #94a3b8; }
  .variant-chips { display: flex; flex-wrap: wrap; gap: 0.6rem; }
  .v-chip {
    background: #1e293b; border: 1px solid #334155; color: #f1f5f9;
    padding: 0.5rem 0.85rem; border-radius: 8px; cursor: pointer;
    display: flex; flex-direction: column; align-items: flex-start; gap: 0.15rem;
    transition: all 0.2s;
  }
  .v-chip:hover { border-color: #38bdf8; }
  .v-chip.selected { border-color: #38bdf8; background: rgba(56, 189, 248, 0.15); box-shadow: 0 0 10px rgba(56, 189, 248, 0.2); }
  .v-price { color: #38bdf8; font-weight: bold; font-size: 0.75rem; }
  .v-stock { color: #64748b; font-size: 0.7rem; }
  .picker-summary {
    background: #1e293b; padding: 0.85rem 1rem; border-radius: 8px;
    display: flex; flex-direction: column; gap: 0.4rem; font-size: 0.9rem;
  }
  .summary-line { display: flex; justify-content: space-between; }
  .picker-price { color: #38bdf8; font-size: 1.1rem; }
  .btn-confirm-var {
    background: #0284c7; color: #fff; border: none; padding: 0.8rem;
    border-radius: 8px; font-size: 0.95rem; font-weight: bold; cursor: pointer;
  }
  .btn-confirm-var:hover { background: #0369a1; }
  .btn-confirm-var:disabled { background: #475569; cursor: not-allowed; }

  /* Product Detail & Reviews Modal Styles */
  .clickable { cursor: pointer; }
  .clickable:hover { opacity: 0.9; }
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

  .card-actions-row { display: flex; gap: 0.5rem; width: 100%; margin-top: 0.5rem; }
  .btn-detail {
    flex: 1; background: #1e293b; color: #cbd5e1; border: 1px solid #334155;
    padding: 0.6rem 0.5rem; border-radius: 6px; font-size: 0.8rem; font-weight: 500;
    cursor: pointer; transition: all 0.2s;
  }
  .btn-detail:hover { background: #334155; color: #fff; }

  .product-detail-modal {
    background: #0f172a; border: 1px solid #334155; border-radius: 12px;
    width: 90%; max-width: 640px; max-height: 88vh; display: flex; flex-direction: column;
    color: #f8fafc; box-shadow: 0 10px 35px rgba(0,0,0,0.7); overflow: hidden;
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
    background: #0284c7; color: #fff; border: none; padding: 0.65rem 1rem;
    border-radius: 6px; font-weight: bold; cursor: pointer;
    transition: background 0.2s;
  }
  .btn-add-detail:hover { background: #0369a1; }
  .btn-add-detail:disabled { background: #475569; cursor: not-allowed; }
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

  /* Reviews & Rating Section in Modal */
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
</style>
