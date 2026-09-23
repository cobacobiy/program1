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
    ProductSuggestionItem,
    FlashSaleItemDto,
  } from './lib/types';
  import { auth } from './lib/auth.svelte';
  import { cart } from './lib/cart.svelte';
  import { toast } from './lib/toast.svelte';
  import { apiFetch } from './lib/api';

  import AuthModal from './lib/AuthModal.svelte';
  import CheckoutModal from './lib/CheckoutModal.svelte';
  import BuyerOrdersModal from './lib/BuyerOrdersModal.svelte';
  import WishlistModal from './lib/WishlistModal.svelte';
  import LiveChat from './lib/LiveChat.svelte';
  import AdminHub from './lib/AdminHub.svelte';
  import FlashSaleBanner from './lib/FlashSaleBanner.svelte';

  import StoreNavbar from './lib/storefront/StoreNavbar.svelte';
  import CategoryFilter from './lib/storefront/CategoryFilter.svelte';
  import ProductGrid from './lib/storefront/ProductGrid.svelte';
  import CartDrawer from './lib/storefront/CartDrawer.svelte';
  import VariantPickerModal from './lib/storefront/VariantPickerModal.svelte';
  import ProductDetailModal from './lib/storefront/ProductDetailModal.svelte';
  import DiagnosticView from './lib/storefront/DiagnosticView.svelte';

  const storeName = 'Program1';

  // Navigation & Modals state
  let activeTab = $state<'store' | 'admin' | 'diagnostic'>('store');
  let isCheckoutOpen = $state(false);
  let isOrdersOpen = $state(false);
  let isWishlistOpen = $state(false);
  let wishlist = $state<WishlistItem[]>([]);
  let wishlistLoading = $state(false);

  // Quick Variant Picker Modal State
  let variantPickerProduct = $state<Product | null>(null);
  let availableVariants = $state<ProductVariant[]>([]);
  let selectedVariant = $state<ProductVariant | null>(null);
  let variantLoading = $state(false);
  let recentlyAddedId = $state<string | null>(null);
  let cartBounceTrigger = $state(false);

  // Review & Rating State
  let ratingSummaries = $state<Record<string, ProductRatingSummary>>({});
  let selectedProductForDetail = $state<Product | null>(null);
  let activeRatingSummary = $state<ProductRatingSummary | null>(null);
  let activeReviews = $state<PublicReview[]>([]);
  let reviewsLoading = $state(false);
  let reviewPage = $state(1);
  let reviewTotalPages = $state(1);
  let reviewTotal = $state(0);

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

  function handleNotificationClick(item: any) {
    if (item.reference_type === 'order') {
      isOrdersOpen = true;
    }
  }

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

  function triggerAddToCartFeedback(productId: string) {
    recentlyAddedId = productId;
    cartBounceTrigger = true;
    setTimeout(() => {
      if (recentlyAddedId === productId) {
        recentlyAddedId = null;
      }
    }, 1100);
    setTimeout(() => {
      cartBounceTrigger = false;
    }, 550);
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
    triggerAddToCartFeedback(prod.id);
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
    triggerAddToCartFeedback(product.id);
  }

  function confirmAddVariantToCart() {
    if (!variantPickerProduct) return;
    const productId = variantPickerProduct.id;
    cart.addItem(variantPickerProduct, 1, selectedVariant || undefined);
    variantPickerProduct = null;
    triggerAddToCartFeedback(productId);
  }

  function handleSelectSuggestionProduct(item: ProductSuggestionItem) {
    const matched = products.find((p) => p.id === item.id);
    if (matched) {
      openProductDetail(matched);
    } else {
      searchQuery = item.name;
    }
  }

  function handleSelectSuggestionKeyword(keyword: string) {
    searchQuery = keyword;
  }

  function handleFlashSaleAddToCart(item: FlashSaleItemDto) {
    const matched = products.find((p) => p.id === item.product_id);
    const prod: Product = matched ? {
      ...matched,
      price_cents: Math.round(item.discount_price * 100),
    } : {
      id: item.product_id,
      name: item.product_name,
      description: '',
      price_cents: Math.round(item.discount_price * 100),
      stock: item.stock_remaining,
      image_url: item.product_image_url,
    };
    cart.addItem(prod);
    triggerAddToCartFeedback(prod.id);
    toast.success(`⚡ ${item.product_name} (Flash Sale) ditambahkan ke keranjang!`);
  }

  function handleSelectFlashSaleProduct(productId: string) {
    const matched = products.find((p) => p.id === productId);
    if (matched) {
      openProductDetail(matched);
    }
  }

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
    if (typeof document !== 'undefined') {
      document.title = `${product.name} | ${storeName || 'Program1'}`;
      window.location.hash = `product-${product.id}`;
    }
    reviewPage = 1;
    activeRatingSummary = ratingSummaries[product.id] || null;
    await Promise.all([
      fetchRatingSummary(product.id),
      fetchProductReviews(product.id, 1),
    ]);
  }

  function closeProductDetail() {
    selectedProductForDetail = null;
    if (typeof document !== 'undefined') {
      document.title = `${storeName || 'Program1'} - Omnichannel Store`;
      if (window.location.hash.startsWith('#product-')) {
        history.pushState('', document.title, window.location.pathname + window.location.search);
      }
    }
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
    fetchCatalog().then(() => {
      const hash = typeof window !== 'undefined' ? window.location.hash : '';
      if (hash.startsWith('#product-')) {
        const pId = hash.replace('#product-', '');
        const found = products.find((p) => p.id === pId);
        if (found) {
          openProductDetail(found);
        } else {
          apiFetch<Product>(`/api/v1/catalog/${pId}`)
            .then((p) => {
              if (p) openProductDetail(p);
            })
            .catch(() => {});
        }
      }
    });
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

  <!-- Modular Navbar -->
  <StoreNavbar
    {activeTab}
    {healthData}
    wishlistCount={wishlist.length}
    {cartBounceTrigger}
    onSelectTab={(tab) => activeTab = tab}
    onOpenWishlist={openWishlistModal}
    onOpenOrders={() => isOrdersOpen = true}
    onOpenCart={() => cart.isOpen = true}
    onNotificationClick={handleNotificationClick}
  />

  <!-- Body Content -->
  <main class="page-body">
    {#if activeTab === 'store'}
      <section class="store-view">
        <!-- Flash Sale Campaign Banner -->
        <FlashSaleBanner
          onAddToCart={handleFlashSaleAddToCart}
          onSelectProduct={handleSelectFlashSaleProduct}
        />

        <!-- Category & Search Filters -->
        <CategoryFilter
          {searchQuery}
          {selectedCategory}
          {categoriesList}
          {totalProductCount}
          onUpdateQuery={(q) => searchQuery = q}
          onSelectCategory={(c) => selectedCategory = c}
          onSelectProduct={handleSelectSuggestionProduct}
          onSelectKeyword={handleSelectSuggestionKeyword}
        />

        <!-- Product Listing Grid -->
        <ProductGrid
          products={filteredProducts}
          {catalogLoading}
          {catalogError}
          {searchQuery}
          {ratingSummaries}
          {recentlyAddedId}
          isProductInWishlist={isProductInWishlist}
          onRetryFetch={fetchCatalog}
          onOpenProductDetail={openProductDetail}
          onToggleWishlist={toggleWishlist}
          onAddToCart={handleAddToCartClick}
        />
      </section>

    {:else if activeTab === 'diagnostic'}
      <DiagnosticView
        {healthData}
        {healthLoading}
        {healthError}
        onRefresh={checkHealth}
      />

    {:else if activeTab === 'admin'}
      <AdminHub />
    {/if}
  </main>

  <!-- Slide-Over Cart Drawer -->
  <CartDrawer
    isOpen={cart.isOpen}
    onClose={() => cart.isOpen = false}
    onStartCheckout={handleStartCheckout}
  />

  <!-- Quick Variant Picker Modal -->
  <VariantPickerModal
    product={variantPickerProduct}
    {availableVariants}
    {selectedVariant}
    onSelectVariant={(v) => selectedVariant = v}
    onConfirm={confirmAddVariantToCart}
    onClose={() => variantPickerProduct = null}
  />

  <!-- Product Detail & Reviews Modal -->
  <ProductDetailModal
    product={selectedProductForDetail}
    {activeRatingSummary}
    {activeReviews}
    {reviewsLoading}
    {reviewPage}
    {reviewTotalPages}
    {reviewTotal}
    {recentlyAddedId}
    isWishlisted={selectedProductForDetail ? isProductInWishlist(selectedProductForDetail.id) : false}
    onClose={closeProductDetail}
    onAddToCart={handleAddToCartClick}
    onToggleWishlist={toggleWishlist}
    onChangeReviewPage={changeReviewPage}
  />

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

  /* Page Body */
  .page-body {
    max-width: 1200px; margin: 0 auto; padding: 2rem 1.5rem; flex: 1; width: 100%; box-sizing: border-box;
  }

  @keyframes slideRight {
    from { transform: translateX(50px); opacity: 0; }
    to { transform: translateX(0); opacity: 1; }
  }
</style>
