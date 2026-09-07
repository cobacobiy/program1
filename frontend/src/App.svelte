<script lang="ts">
  import { onMount } from 'svelte';
  import type { HealthResponse, Product, CatalogPageResponse, CartItem } from './lib/types';
  import { formatRupiah } from './lib/currency';

  // Svelte 5 Runes state
  let activeTab = $state<'store' | 'diagnostic'>('store');
  let healthData = $state<HealthResponse | null>(null);
  let healthLoading = $state(false);
  let healthError = $state<string | null>(null);

  let products = $state<Product[]>([]);
  let catalogLoading = $state(false);
  let catalogError = $state<string | null>(null);
  let searchQuery = $state('');
  let sortBy = $state('default');

  let cart = $state<CartItem[]>([]);
  let isCartOpen = $state(false);
  let toastMsg = $state<string | null>(null);

  // Derived calculations
  const totalCartItems = $derived(cart.reduce((sum, item) => sum + item.quantity, 0));
  const totalCartPrice = $derived(cart.reduce((sum, item) => sum + (item.product.price_cents * item.quantity), 0));

  const filteredProducts = $derived(
    products.filter(p => p.name.toLowerCase().includes(searchQuery.toLowerCase()))
  );

  function showToast(msg: string) {
    toastMsg = msg;
    setTimeout(() => {
      toastMsg = null;
    }, 3000);
  }

  async function checkHealth() {
    healthLoading = true;
    healthError = null;
    try {
      const res = await fetch('/health');
      if (!res.ok) throw new Error(`HTTP ${res.status}: ${res.statusText}`);
      healthData = await res.json();
    } catch (err: any) {
      healthError = err.message || 'Gagal terhubung ke backend Axum';
    } finally {
      healthLoading = false;
    }
  }

  async function fetchCatalog() {
    catalogLoading = true;
    catalogError = null;
    try {
      const res = await fetch('/catalog?page=1&page_size=20');
      if (!res.ok) throw new Error(`HTTP ${res.status}: ${res.statusText}`);
      const data: CatalogPageResponse = await res.json();
      products = data.items || [];
    } catch (err: any) {
      catalogError = err.message || 'Gagal memuat katalog produk';
    } finally {
      catalogLoading = false;
    }
  }

  function addToCart(product: Product) {
    if (product.stock <= 0) {
      showToast('⚠️ Stok produk ini sudah habis!');
      return;
    }

    const idx = cart.findIndex(c => c.product.id === product.id);
    if (idx > -1) {
      if (cart[idx].quantity >= product.stock) {
        showToast(`⚠️ Maksimal stok tersedia ${product.stock} unit.`);
        return;
      }
      cart[idx].quantity += 1;
    } else {
      cart = [...cart, { product, quantity: 1 }];
    }
    showToast(`✅ ${product.name} dimasukkan ke keranjang!`);
  }

  function updateQuantity(productId: string, delta: number) {
    const item = cart.find(c => c.product.id === productId);
    if (!item) return;

    const next = item.quantity + delta;
    if (next <= 0) {
      cart = cart.filter(c => c.product.id !== productId);
    } else if (next > item.product.stock) {
      showToast(`⚠️ Maksimal stok tersedia ${item.product.stock} unit.`);
    } else {
      item.quantity = next;
    }
  }

  onMount(() => {
    checkHealth();
    fetchCatalog();
  });
</script>

<div class="app-wrapper">
  <!-- Toast Notification -->
  {#if toastMsg}
    <div class="toast-popup">
      {toastMsg}
    </div>
  {/if}

  <!-- Header -->
  <header class="navbar">
    <div class="nav-container">
      <div class="brand">
        <span class="logo-icon">⚡</span>
        <div>
          <h1>Program1</h1>
          <span class="brand-tag">Svelte 5 + Bun SPA</span>
        </div>
      </div>

      <nav class="nav-links">
        <button class:active={activeTab === 'store'} onclick={() => activeTab = 'store'}>
          🏪 Katalog Produk
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

      <div class="nav-actions">
        <button class="cart-btn" onclick={() => isCartOpen = true}>
          🛒 Keranjang
          {#if totalCartItems > 0}
            <span class="badge-count">{totalCartItems}</span>
          {/if}
        </button>
      </div>
    </div>
  </header>

  <!-- Main Container -->
  <main class="main-body">
    {#if activeTab === 'store'}
      <section class="store-section">
        <div class="toolbar">
          <div class="search-box">
            <span class="search-icon">🔍</span>
            <input
              type="text"
              placeholder="Cari produk di katalog..."
              bind:value={searchQuery}
            />
          </div>
          <button class="btn-refresh" onclick={fetchCatalog} disabled={catalogLoading}>
            {catalogLoading ? 'Memuat...' : 'Muat Ulang'}
          </button>
        </div>

        {#if catalogLoading}
          <div class="loading-grid">
            {#each Array(6) as _}
              <div class="skeleton-card"></div>
            {/each}
          </div>
        {:else if catalogError}
          <div class="alert-box error">
            <p><strong>Gagal Memuat Katalog:</strong> {catalogError}</p>
            <p class="hint">Pastikan backend Rust Axum berjalan di port 8080 (<code>cargo run -p program1-web</code>).</p>
          </div>
        {:else if filteredProducts.length === 0}
          <div class="empty-state">
            <p>Tidak ada produk yang cocok dengan pencarian "{searchQuery}".</p>
          </div>
        {:else}
          <div class="product-grid">
            {#each filteredProducts as product (product.id)}
              <div class="product-card">
                <div class="img-box">
                  {#if product.image_url}
                    <img src={product.image_url} alt={product.name} />
                  {:else}
                    <span class="placeholder-icon">📦</span>
                  {/if}
                  {#if product.stock <= 0}
                    <span class="out-badge">Habis</span>
                  {/if}
                </div>
                <div class="card-body">
                  <h3 class="product-name">{product.name}</h3>
                  <p class="product-desc">{product.description || 'Produk berkualitas tinggi'}</p>
                  <div class="card-foot">
                    <span class="price-tag">{formatRupiah(product.price_cents)}</span>
                    <small class="stock-text">Stok: {product.stock}</small>
                  </div>
                  <button
                    class="btn-buy"
                    disabled={product.stock <= 0}
                    onclick={() => addToCart(product)}
                  >
                    {product.stock <= 0 ? 'Habis' : '+ Keranjang'}
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </section>

    {:else if activeTab === 'diagnostic'}
      <section class="diagnostic-section">
        <h2>🛠️ Backend Connection & Diagnostics</h2>
        <p class="section-desc">
          Frontend Svelte 5 ini berkomunikasi langsung dengan Modular Monolith Rust Axum via JSON REST API.
        </p>

        <div class="diag-card">
          <div class="diag-header">
            <h3>Pemeriksaan Endpoint <code>GET /health</code></h3>
            <button class="btn-primary" onclick={checkHealth} disabled={healthLoading}>
              {healthLoading ? 'Memeriksa...' : 'Ping Ulang'}
            </button>
          </div>

          {#if healthLoading}
            <p class="text-muted">Menghubungi server Axum...</p>
          {:else if healthError}
            <div class="alert-box error">
              <h4>❌ Koneksi Gagal</h4>
              <p>{healthError}</p>
              <small>Vite dev server mencoba proxy ke <code>http://127.0.0.1:8080/health</code></small>
            </div>
          {:else if healthData}
            <div class="alert-box success">
              <h4>✅ Server Rust Axum Terhubung & Aktif!</h4>
              <ul class="diag-list">
                <li><strong>Status Server:</strong> <code>{healthData.status}</code></li>
                <li><strong>Versi Monolith:</strong> <code>{healthData.version}</code></li>
                {#if healthData.subsystems}
                  <li><strong>Subsystem Status:</strong>
                    <pre>{JSON.stringify(healthData.subsystems, null, 2)}</pre>
                  </li>
                {/if}
              </ul>
            </div>
          {/if}
        </div>
      </section>
    {/if}
  </main>

  <!-- Cart Drawer -->
  {#if isCartOpen}
    <div class="drawer-backdrop" onclick={() => isCartOpen = false}>
      <div class="drawer" onclick={(e) => e.stopPropagation()}>
        <div class="drawer-head">
          <h3>🛒 Keranjang Belanja ({totalCartItems})</h3>
          <button class="btn-close" onclick={() => isCartOpen = false}>&times;</button>
        </div>

        <div class="drawer-content">
          {#if cart.length === 0}
            <div class="empty-cart">
              <p>Keranjang belanja kosong.</p>
              <button class="btn-secondary" onclick={() => isCartOpen = false}>Lihat Produk</button>
            </div>
          {:else}
            <div class="cart-items-list">
              {#each cart as item (item.product.id)}
                <div class="cart-card">
                  <div class="cart-info">
                    <h4>{item.product.name}</h4>
                    <span class="cart-price">{formatRupiah(item.product.price_cents)}</span>
                  </div>
                  <div class="cart-ctrls">
                    <button class="btn-step" onclick={() => updateQuantity(item.product.id, -1)}>-</button>
                    <span class="cart-qty">{item.quantity}</span>
                    <button class="btn-step" onclick={() => updateQuantity(item.product.id, 1)}>+</button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>

        {#if cart.length > 0}
          <div class="drawer-foot">
            <div class="total-row">
              <span>Total Tagihan:</span>
              <strong class="total-val">{formatRupiah(totalCartPrice)}</strong>
            </div>
            <button class="btn-checkout" onclick={() => showToast('🚀 Fitur Checkout Midtrans siap diintegrasikan di Issue 14!')}>
              Lanjut ke Pembayaran 💳
            </button>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  :global(body) {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
    background: #090d16;
    color: #f1f5f9;
  }

  .app-wrapper {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }

  .toast-popup {
    position: fixed;
    bottom: 2rem;
    left: 50%;
    transform: translateX(-50%);
    background: #0284c7;
    color: #fff;
    padding: 0.75rem 1.5rem;
    border-radius: 9999px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    z-index: 9999;
    font-weight: 500;
    animation: fadeIn 0.2s ease-out;
  }

  /* Navbar */
  .navbar {
    background: #0f172a;
    border-bottom: 1px solid #1e293b;
    position: sticky;
    top: 0;
    z-index: 50;
  }
  .nav-container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 0.75rem 1.5rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }
  .logo-icon { font-size: 1.8rem; }
  .brand h1 { margin: 0; font-size: 1.25rem; color: #38bdf8; }
  .brand-tag { font-size: 0.75rem; color: #94a3b8; }
  .nav-links { display: flex; gap: 0.5rem; }
  .nav-links button {
    background: transparent;
    border: none;
    color: #94a3b8;
    padding: 0.5rem 1rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.95rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    transition: all 0.2s;
  }
  .nav-links button:hover { background: #1e293b; color: #f8fafc; }
  .nav-links button.active { background: #1e293b; color: #38bdf8; font-weight: 600; }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  .status-dot.green { background: #10b981; box-shadow: 0 0 6px #10b981; }
  .status-dot.yellow { background: #f59e0b; }

  .cart-btn {
    background: #0284c7;
    color: #fff;
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 6px;
    font-weight: 600;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    position: relative;
  }
  .badge-count {
    background: #ef4444;
    color: white;
    font-size: 0.75rem;
    padding: 0.1rem 0.4rem;
    border-radius: 9999px;
  }

  /* Main Body */
  .main-body {
    max-width: 1200px;
    margin: 0 auto;
    padding: 2rem 1.5rem;
    flex: 1;
    width: 100%;
    box-sizing: border-box;
  }

  .toolbar {
    display: flex;
    gap: 1rem;
    margin-bottom: 1.5rem;
  }
  .search-box {
    flex: 1;
    display: flex;
    align-items: center;
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 8px;
    padding: 0 0.8rem;
  }
  .search-icon { font-size: 1rem; margin-right: 0.5rem; }
  .search-box input {
    width: 100%;
    background: transparent;
    border: none;
    padding: 0.75rem 0;
    color: #fff;
    font-size: 0.95rem;
  }
  .search-box input:focus { outline: none; }
  .btn-refresh {
    background: #334155;
    color: #fff;
    border: none;
    padding: 0 1.25rem;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 500;
  }

  /* Product Grid */
  .product-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 1.5rem;
  }
  .product-card {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 12px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    transition: transform 0.2s, box-shadow 0.2s, border-color 0.2s;
  }
  .product-card:hover {
    transform: translateY(-4px);
    box-shadow: 0 10px 24px rgba(0,0,0,0.35);
    border-color: #0284c7;
  }
  .img-box {
    height: 180px;
    background: #0f172a;
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .img-box img { width: 100%; height: 100%; object-fit: cover; }
  .placeholder-icon { font-size: 3.5rem; }
  .out-badge {
    position: absolute;
    top: 8px;
    right: 8px;
    background: #dc2626;
    color: white;
    font-size: 0.75rem;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-weight: bold;
  }
  .card-body {
    padding: 1.25rem;
    flex: 1;
    display: flex;
    flex-direction: column;
  }
  .product-name {
    margin: 0 0 0.5rem;
    font-size: 1.05rem;
    color: #f8fafc;
  }
  .product-desc {
    font-size: 0.85rem;
    color: #94a3b8;
    margin: 0 0 1rem;
    flex: 1;
  }
  .card-foot {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }
  .price-tag { font-size: 1.15rem; font-weight: bold; color: #38bdf8; }
  .stock-text { color: #64748b; }
  .btn-buy {
    width: 100%;
    background: #0284c7;
    color: #fff;
    border: none;
    padding: 0.65rem;
    border-radius: 6px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
  }
  .btn-buy:hover:not(:disabled) { background: #0369a1; }
  .btn-buy:disabled { background: #475569; cursor: not-allowed; }

  /* Diagnostic Card */
  .diag-card {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 12px;
    padding: 1.5rem;
  }
  .diag-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }
  .alert-box {
    padding: 1.25rem;
    border-radius: 8px;
    margin-top: 1rem;
  }
  .alert-box.success { background: #064e3b; color: #a7f3d0; }
  .alert-box.error { background: #7f1d1d; color: #fecaca; }
  .diag-list { list-style: none; padding: 0; margin: 0.5rem 0; display: flex; flex-direction: column; gap: 0.4rem; }

  /* Drawer */
  .drawer-backdrop {
    position: fixed; inset: 0; background: rgba(0,0,0,0.6); z-index: 200;
    display: flex; justify-content: flex-end;
  }
  .drawer {
    width: 100%; max-width: 400px; height: 100%;
    background: #0f172a; border-left: 1px solid #1e293b;
    display: flex; flex-direction: column; animation: slideIn 0.2s ease-out;
  }
  .drawer-head {
    padding: 1.25rem; border-bottom: 1px solid #1e293b;
    display: flex; justify-content: space-between; align-items: center;
  }
  .btn-close { background: none; border: none; font-size: 1.5rem; color: #fff; cursor: pointer; }
  .drawer-content { flex: 1; overflow-y: auto; padding: 1.25rem; }
  .cart-card {
    background: #1e293b; padding: 0.75rem 1rem; border-radius: 8px;
    display: flex; justify-content: space-between; align-items: center;
    margin-bottom: 0.75rem;
  }
  .cart-info h4 { margin: 0 0 0.25rem; font-size: 0.95rem; }
  .cart-price { color: #38bdf8; font-weight: bold; font-size: 0.9rem; }
  .cart-ctrls { display: flex; align-items: center; gap: 0.5rem; }
  .btn-step {
    background: #334155; color: white; border: none; width: 26px; height: 26px;
    border-radius: 4px; cursor: pointer; font-weight: bold;
  }
  .drawer-foot {
    padding: 1.25rem; border-top: 1px solid #1e293b; background: #0b1120;
  }
  .total-row { display: flex; justify-content: space-between; font-size: 1.1rem; margin-bottom: 1rem; }
  .total-val { color: #38bdf8; }
  .btn-checkout {
    width: 100%; background: #0284c7; color: #fff; border: none;
    padding: 0.8rem; border-radius: 8px; font-size: 1rem; font-weight: bold; cursor: pointer;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translate(-50%, 10px); }
    to { opacity: 1; transform: translate(-50%, 0); }
  }
  @keyframes slideIn {
    from { transform: translateX(100%); }
    to { transform: translateX(0); }
  }
</style>
