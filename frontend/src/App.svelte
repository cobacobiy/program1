<script lang="ts">
  import { onMount } from 'svelte';
  import type { HealthResponse, Product, CatalogPageResponse } from './lib/types';
  import { formatRupiah } from './lib/currency';
  import { auth } from './lib/auth.svelte';
  import { cart } from './lib/cart.svelte';
  import { toast } from './lib/toast.svelte';
  import { apiFetch } from './lib/api';

  import AuthModal from './lib/AuthModal.svelte';
  import CheckoutModal from './lib/CheckoutModal.svelte';
  import BuyerOrdersModal from './lib/BuyerOrdersModal.svelte';
  import LiveChat from './lib/LiveChat.svelte';
  import AdminHub from './lib/AdminHub.svelte';

  // Navigation & Modals state
  let activeTab = $state<'store' | 'admin' | 'diagnostic'>('store');
  let isCheckoutOpen = $state(false);
  let isOrdersOpen = $state(false);

  // Health state
  let healthData = $state<HealthResponse | null>(null);
  let healthLoading = $state(false);
  let healthError = $state<string | null>(null);

  // Catalog state
  let products = $state<Product[]>([]);
  let catalogLoading = $state(false);
  let catalogError = $state<string | null>(null);
  let searchQuery = $state('');
  let selectedCategory = $state<string>('all');

  const categories = $derived(
    ['all', ...Array.from(new Set(products.map(p => p.category).filter(Boolean)))] as string[]
  );

  const filteredProducts = $derived(
    products.filter(p => {
      const matchSearch = p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
                          p.description?.toLowerCase().includes(searchQuery.toLowerCase());
      const matchCategory = selectedCategory === 'all' || p.category === selectedCategory;
      return matchSearch && matchCategory;
    })
  );

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
    } catch (err: any) {
      catalogError = err.message || 'Gagal memuat katalog produk';
    } finally {
      catalogLoading = false;
    }
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
          🏪 Katalog Toko
        </button>
        <button class:active={activeTab === 'admin'} onclick={() => activeTab = 'admin'}>
          ⚙️ Admin Hub
        </button>
        <button class:active={activeTab === 'diagnostic'} onclick={() => activeTab = 'diagnostic'}>
          🔍 Server Health
          {#if healthData}
            <span class="status-dot green"></span>
          {:else}
            <span class="status-dot yellow"></span>
          {/if}
        </button>
      </nav>

      <div class="nav-user-actions">
        {#if auth.user}
          <button class="btn-orders" onclick={() => isOrdersOpen = true}>
            📦 Pesanan Saya
          </button>
          <div class="user-pill">
            <span class="uname">👤 {auth.user.name}</span>
            <button class="btn-logout" onclick={() => auth.logout()}>Keluar</button>
          </div>
        {:else}
          <button class="btn-login" onclick={() => auth.isModalOpen = true}>
            Masuk / Daftar
          </button>
        {/if}

        <button class="cart-trigger" onclick={() => cart.isOpen = true}>
          🛒 Keranjang
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
              placeholder="Cari produk impianmu..."
              bind:value={searchQuery}
            />
          </div>

          {#if categories.length > 1}
            <div class="category-pills">
              {#each categories as cat}
                <button
                  class="pill-btn"
                  class:active={selectedCategory === cat}
                  onclick={() => selectedCategory = cat}
                >
                  {cat === 'all' ? 'Semua Kategori' : cat}
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
            <p><strong>Gagal Memuat Produk:</strong> {catalogError}</p>
            <button class="btn-retry" onclick={fetchCatalog}>Coba Lagi</button>
          </div>
        {:else if filteredProducts.length === 0}
          <div class="empty-box">
            <p>Produk tidak ditemukan untuk pencarian "{searchQuery}".</p>
          </div>
        {:else}
          <div class="catalog-grid">
            {#each filteredProducts as product (product.id)}
              <div class="product-item">
                <div class="img-container">
                  {#if product.image_url}
                    <img src={product.image_url} alt={product.name} loading="lazy" />
                  {:else}
                    <span class="placeholder-emoji">📦</span>
                  {/if}
                  {#if product.stock <= 0}
                    <span class="badge-habis">Habis</span>
                  {/if}
                </div>

                <div class="item-details">
                  <h3 class="p-title">{product.name}</h3>
                  <p class="p-desc">{product.description || 'Produk kualitas terjamin dari katalog.'}</p>
                  
                  <div class="item-meta">
                    <span class="price-value">{formatRupiah(product.price_cents)}</span>
                    <small class="stock-value">Stok: {product.stock}</small>
                  </div>

                  <button
                    class="btn-add"
                    disabled={product.stock <= 0}
                    onclick={() => cart.addItem(product)}
                  >
                    {product.stock <= 0 ? 'Stok Habis' : '+ Keranjang'}
                  </button>
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
          <h3>🛒 Keranjang ({cart.totalItems})</h3>
          <button class="close-drawer" onclick={() => cart.isOpen = false}>&times;</button>
        </div>

        <div class="drawer-body">
          {#if cart.items.length === 0}
            <div class="empty-cart-state">
              <p>Keranjang Anda masih kosong.</p>
              <button class="btn-explore" onclick={() => cart.isOpen = false}>Mulai Belanja</button>
            </div>
          {:else}
            <div class="cart-lines">
              {#each cart.items as item (item.product.id)}
                <div class="line-item">
                  <div class="line-info">
                    <h4>{item.product.name}</h4>
                    <span class="line-price">{formatRupiah(item.product.price_cents)}</span>
                  </div>
                  <div class="line-actions">
                    <button class="qty-btn" onclick={() => cart.updateQuantity(item.product.id, -1)}>-</button>
                    <span class="qty-num">{item.quantity}</span>
                    <button class="qty-btn" onclick={() => cart.updateQuantity(item.product.id, 1)}>+</button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>

        {#if cart.items.length > 0}
          <div class="drawer-footer">
            <div class="total-bar">
              <span>Total:</span>
              <strong class="total-text">{formatRupiah(cart.totalAmountCents)}</strong>
            </div>
            <button class="btn-checkout" onclick={handleStartCheckout}>
              Lanjut ke Pembayaran 💳
            </button>
          </div>
        {/if}
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
    padding: 0.4rem 0.8rem; border-radius: 20px; font-size: 0.85rem; cursor: pointer;
  }
  .pill-btn.active { background: #0284c7; color: #fff; border-color: #0284c7; font-weight: bold; }

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
  .item-details { padding: 1.25rem; flex: 1; display: flex; flex-direction: column; }
  .p-title { margin: 0 0 0.5rem; font-size: 1.05rem; color: #f8fafc; }
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
</style>
