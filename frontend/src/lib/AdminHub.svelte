<script lang="ts">
  import { onMount } from 'svelte';
  import { adminAuth } from './adminAuth.svelte';
  import { toast } from './toast.svelte';
  import { formatRupiah } from './currency';
  import type { Product, ProductVariant, CreateVariantPayload, Coupon, CreateCouponPayload, DiscountType } from './types';

  interface AdminOrder {
    id: string;
    total_amount_cents: number;
    status: string;
    created_at: string;
    tracking_number?: string | null;
  }

  interface InventoryRecord {
    id: string;
    product_name?: string;
    stock: number;
    safety_stock?: number;
  }

  interface AuditLogRecord {
    id: string;
    actor_username?: string;
    action: string;
    resource_type: string;
    created_at: string;
  }

  // Active sub-tab
  let subTab = $state<'kpi' | 'catalog' | 'inventory' | 'orders' | 'audit' | 'coupons'>('kpi');

  // Login form state
  let loginUser = $state('admin');
  let loginPass = $state('admin123');

  // Data states
  let products = $state<Product[]>([]);
  let orders = $state<AdminOrder[]>([]);
  let inventory = $state<InventoryRecord[]>([]);
  let auditLogs = $state<AuditLogRecord[]>([]);
  let coupons = $state<Coupon[]>([]);
  let loading = $state(false);

  // New coupon modal state
  let isAddCouponOpen = $state(false);
  let newCouponCode = $state('');
  let newCouponDiscountType = $state<DiscountType>('PERCENTAGE');
  let newCouponDiscountVal = $state(10);
  let newCouponMinOrder = $state(0);
  let newCouponMaxCap = $state<number | null>(null);
  let newCouponLimit = $state<number | null>(null);
  let newCouponStartDate = $state(new Date().toISOString().slice(0, 16));
  let newCouponEndDate = $state(new Date(Date.now() + 30 * 86400000).toISOString().slice(0, 16));
  let isCreatingCoupon = $state(false);

  // New product modal state
  let isAddProductOpen = $state(false);
  let newName = $state('');
  let newDesc = $state('');
  let newPrice = $state(50000);
  let newStock = $state(20);
  let newCategory = $state('Umum');
  let selectedFile = $state<File | null>(null);
  let isUploading = $state(false);

  // Variant Modal State
  let selectedProductForVariants = $state<Product | null>(null);
  let variants = $state<ProductVariant[]>([]);
  let variantsLoading = $state(false);
  let varName = $state('Ukuran');
  let varValue = $state('');
  let varSku = $state('');
  let varPriceOverride = $state<number | null>(null);
  let varStock = $state(10);
  let varSaving = $state(false);

  async function openVariantModal(prod: Product) {
    selectedProductForVariants = prod;
    varValue = '';
    varSku = '';
    varPriceOverride = null;
    await fetchVariants(prod.id);
  }

  async function fetchVariants(productId: string) {
    variantsLoading = true;
    try {
      variants = await fetchAdmin<ProductVariant[]>(`/api/v1/catalog/${productId}/variants`);
    } catch (err: any) {
      toast.error(err.message || 'Gagal mengambil daftar varian');
    } finally {
      variantsLoading = false;
    }
  }

  async function handleAddVariant() {
    if (!selectedProductForVariants) return;
    if (!varName.trim() || !varValue.trim()) {
      toast.error('Nama dan Nilai varian wajib diisi!');
      return;
    }

    varSaving = true;
    try {
      const payload: CreateVariantPayload = {
        variant_name: varName.trim(),
        variant_value: varValue.trim(),
        sku: varSku.trim() || null,
        price_override: varPriceOverride && varPriceOverride > 0 ? varPriceOverride : null,
        stock_quantity: Number(varStock) || 0,
      };

      await fetchAdmin(`/api/v1/catalog/${selectedProductForVariants.id}/variants`, {
        method: 'POST',
        body: JSON.stringify(payload),
      });

      toast.success('Varian berhasil ditambahkan!');
      varValue = '';
      varSku = '';
      varPriceOverride = null;
      await fetchVariants(selectedProductForVariants.id);
    } catch (err: any) {
      toast.error(err.message || 'Gagal menambahkan varian');
    } finally {
      varSaving = false;
    }
  }

  async function handleDeleteVariant(variantId: string) {
    if (!selectedProductForVariants) return;
    if (!confirm('Hapus varian ini?')) return;

    try {
      await fetchAdmin(`/api/v1/catalog/${selectedProductForVariants.id}/variants/${variantId}`, {
        method: 'DELETE',
      });
      toast.success('Varian berhasil dihapus!');
      await fetchVariants(selectedProductForVariants.id);
    } catch (err: any) {
      toast.error(err.message || 'Gagal menghapus varian');
    }
  }

  async function handleToggleCoupon(couponId: string) {
    try {
      const updated = await fetchAdmin<Coupon>(`/api/v1/admin/coupons/${couponId}/toggle`, {
        method: 'PUT',
      });
      coupons = coupons.map(c => c.id === couponId ? updated : c);
      toast.success(`Status kupon ${updated.code} diubah!`);
    } catch (err: any) {
      toast.error(err.message || 'Gagal mengubah status kupon');
    }
  }

  async function handleDeleteCoupon(couponId: string, code: string) {
    if (!confirm(`Hapus kupon "${code}"? Tindakan ini tidak dapat dibatalkan.`)) return;
    try {
      await fetchAdmin(`/api/v1/admin/coupons/${couponId}`, {
        method: 'DELETE',
      });
      coupons = coupons.filter(c => c.id !== couponId);
      toast.success(`Kupon "${code}" berhasil dihapus.`);
    } catch (err: any) {
      toast.error(err.message || 'Gagal menghapus kupon');
    }
  }

  async function handleCreateCoupon(e: Event) {
    e.preventDefault();
    if (!newCouponCode.trim()) {
      toast.error('Kode kupon wajib diisi!');
      return;
    }

    isCreatingCoupon = true;
    try {
      const payload: CreateCouponPayload = {
        code: newCouponCode.trim().toUpperCase(),
        discount_type: newCouponDiscountType,
        discount_value: Number(newCouponDiscountVal),
        min_order_amount: newCouponMinOrder ? Number(newCouponMinOrder) : 0,
        max_discount_amount: newCouponMaxCap && newCouponMaxCap > 0 ? Number(newCouponMaxCap) : undefined,
        usage_limit: newCouponLimit && newCouponLimit > 0 ? Number(newCouponLimit) : undefined,
        start_date: new Date(newCouponStartDate).toISOString(),
        end_date: new Date(newCouponEndDate).toISOString(),
      };

      const created = await fetchAdmin<Coupon>('/api/v1/admin/coupons', {
        method: 'POST',
        body: JSON.stringify(payload),
      });

      coupons = [created, ...coupons];
      toast.success(`Kupon "${created.code}" berhasil dibuat!`);
      isAddCouponOpen = false;
      newCouponCode = '';
      newCouponDiscountVal = 10;
      newCouponMaxCap = null;
      newCouponLimit = null;
    } catch (err: any) {
      toast.error(err.message || 'Gagal membuat kupon.');
    } finally {
      isCreatingCoupon = false;
    }
  }

  async function fetchAdmin<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
    const headers = new Headers(options.headers || {});
    if (adminAuth.token) {
      headers.set('Authorization', `Bearer ${adminAuth.token}`);
    }
    if (!headers.has('Content-Type') && !(options.body instanceof FormData)) {
      headers.set('Content-Type', 'application/json');
    }

    const res = await fetch(endpoint, { ...options, headers });
    if (res.status === 401) {
      adminAuth.logout();
      throw new Error('Sesi admin berakhir.');
    }
    const data = await res.json().catch(() => ({}));
    if (!res.ok) {
      throw new Error(data.message || `Request failed (${res.status})`);
    }
    return data as T;
  }

  async function loadData() {
    if (!adminAuth.token) return;
    loading = true;
    try {
      if (subTab === 'kpi' || subTab === 'catalog') {
        const cat = await fetchAdmin<{ items: Product[] }>('/catalog?page=1&page_size=50').catch(() => ({ items: [] }));
        products = cat.items || [];
      }
      if (subTab === 'kpi' || subTab === 'orders') {
        const ord = await fetchAdmin<AdminOrder[]>('/orders').catch(() => []);
        orders = ord || [];
      }
      if (subTab === 'inventory') {
        const inv = await fetchAdmin<InventoryRecord[]>('/inventory').catch(() => []);
        inventory = inv || [];
      }
      if (subTab === 'audit') {
        const aud = await fetchAdmin<AuditLogRecord[]>('/audit/logs').catch(() => []);
        auditLogs = aud || [];
      }
      if (subTab === 'kpi' || subTab === 'coupons') {
        const coup = await fetchAdmin<Coupon[]>('/api/v1/admin/coupons').catch(() => []);
        coupons = coup || [];
      }
    } catch (err: any) {
      toast.error(err.message || 'Gagal memuat data admin');
    } finally {
      loading = false;
    }
  }

  async function handleCreateProduct(e: Event) {
    e.preventDefault();
    isUploading = true;

    try {
      let imageUrl: string | null = null;

      // 1. Upload cover image jika ada
      if (selectedFile) {
        const form = new FormData();
        form.append('image', selectedFile);
        const upRes = await fetch('/v1/upload', {
          method: 'POST',
          headers: { Authorization: `Bearer ${adminAuth.token}` },
          body: form,
        });
        if (upRes.ok) {
          const upData = await upRes.json();
          imageUrl = upData.url;
        }
      }

      // 2. Buat produk di katalog
      await fetchAdmin('/api/v1/catalog', {
        method: 'POST',
        body: JSON.stringify({
          name: newName,
          description: newDesc,
          price_cents: newPrice,
          stock: newStock,
          category: newCategory,
          image_url: imageUrl,
        }),
      }).catch(async () => {
        // Fallback endpoint
        return fetchAdmin('/catalog', {
          method: 'POST',
          body: JSON.stringify({
            name: newName,
            description: newDesc,
            price_cents: newPrice,
            stock: newStock,
            category: newCategory,
            image_url: imageUrl,
          }),
        });
      });

      toast.success(`Produk "${newName}" berhasil ditambahkan!`);
      isAddProductOpen = false;
      newName = '';
      newDesc = '';
      selectedFile = null;
      await loadData();
    } catch (err: any) {
      toast.error(err.message || 'Gagal membuat produk.');
    } finally {
      isUploading = false;
    }
  }

  async function updateOrderStatus(orderId: string, nextStatus: string) {
    let trackingNumber: string | null = null;
    if (nextStatus === 'SHIPPED') {
      trackingNumber = prompt('Masukkan Nomor Resi / Tracking Pengiriman:');
      if (!trackingNumber) return;
    }

    try {
      await fetchAdmin(`/orders/${orderId}/status`, {
        method: 'PUT',
        body: JSON.stringify({
          status: nextStatus,
          tracking_number: trackingNumber,
        }),
      });
      toast.success(`Status pesanan #${orderId.slice(0, 8)} diubah ke ${nextStatus}`);
      await loadData();
    } catch (err: any) {
      toast.error(err.message || 'Gagal mengubah status pesanan');
    }
  }

  onMount(() => {
    if (adminAuth.token) {
      loadData();
    }
  });

  $effect(() => {
    if (adminAuth.token && subTab) {
      loadData();
    }
  });
</script>

<div class="admin-wrapper">
  {#if !adminAuth.token}
    <!-- Admin Login Screen -->
    <div class="admin-login-card">
      <div class="login-header">
        <span class="lock-icon">🔒</span>
        <h2>Program1 Admin Hub</h2>
        <p>Masuk untuk mengelola produk, inventori, dan pemrosesan pesanan.</p>
      </div>

      <form onsubmit={(e) => { e.preventDefault(); adminAuth.login(loginUser, loginPass); }} class="login-form">
        <label>
          Username Admin:
          <input type="text" bind:value={loginUser} required />
        </label>
        <label>
          Password:
          <input type="password" bind:value={loginPass} required />
        </label>

        <button type="submit" class="btn-submit" disabled={adminAuth.isLoading}>
          {adminAuth.isLoading ? 'Memverifikasi...' : 'Masuk ke Dashboard'}
        </button>
      </form>
    </div>
  {:else}
    <!-- Admin Dashboard -->
    <div class="admin-container">
      <aside class="sidebar">
        <div class="side-header">
          <h3>⚡ Admin Hub</h3>
          <span class="admin-name">User: <strong>{adminAuth.username}</strong></span>
        </div>

        <nav class="side-nav">
          <button class:active={subTab === 'kpi'} onclick={() => subTab = 'kpi'}>📊 Ringkasan KPI</button>
          <button class:active={subTab === 'catalog'} onclick={() => subTab = 'catalog'}>📦 Katalog Produk ({products.length})</button>
          <button class:active={subTab === 'inventory'} onclick={() => subTab = 'inventory'}>🏭 Stok & Inventori</button>
          <button class:active={subTab === 'orders'} onclick={() => subTab = 'orders'}>🛒 Pesanan Masuk ({orders.length})</button>
          <button class:active={subTab === 'coupons'} onclick={() => subTab = 'coupons'}>🏷️ Kupon Diskon ({coupons.length})</button>
          <button class:active={subTab === 'audit'} onclick={() => subTab = 'audit'}>📜 Audit Logs</button>
        </nav>

        <button class="btn-logout" onclick={() => adminAuth.logout()}>🚪 Keluar</button>
      </aside>

      <section class="admin-main">
        {#if subTab === 'kpi'}
          <div class="kpi-panel">
            <h2>📊 Ringkasan Kinerja Toko</h2>
            <div class="kpi-grid">
              <div class="kpi-card">
                <span class="kpi-label">Total Produk Aktif</span>
                <strong class="kpi-val">{products.length}</strong>
                <small class="kpi-hint">Tersedia di etalase pembeli</small>
              </div>
              <div class="kpi-card">
                <span class="kpi-label">Total Pesanan Masuk</span>
                <strong class="kpi-val">{orders.length}</strong>
                <small class="kpi-hint">Seluruh transaksi pembeli</small>
              </div>
              <div class="kpi-card">
                <span class="kpi-label">Total Omset Penjualan</span>
                <strong class="kpi-val">
                  {formatRupiah(orders.reduce((sum, o) => sum + o.total_amount_cents, 0))}
                </strong>
                <small class="kpi-hint">Akumulasi nilai pesanan</small>
              </div>
              <div class="kpi-card">
                <span class="kpi-label">Kupon Promo Aktif</span>
                <strong class="kpi-val">{coupons.filter(c => c.is_active).length}</strong>
                <small class="kpi-hint">Dari total {coupons.length} voucher</small>
              </div>
            </div>
          </div>

        {:else if subTab === 'catalog'}
          <div class="section-panel">
            <div class="panel-top">
              <h2>📦 Manajemen Katalog Produk</h2>
              <button class="btn-add-prod" onclick={() => isAddProductOpen = true}>+ Tambah Produk Baru</button>
            </div>

            {#if loading}
              <p>Memuat katalog...</p>
            {:else}
              <table class="table-custom">
                <thead>
                  <tr>
                    <th>Gambar</th>
                    <th>Nama Produk</th>
                    <th>Kategori</th>
                    <th>Harga</th>
                    <th>Stok</th>
                    <th>Varian</th>
                  </tr>
                </thead>
                <tbody>
                  {#each products as p (p.id)}
                    <tr>
                      <td class="td-img">
                        {#if p.image_url}
                          <img src={p.image_url} alt={p.name} />
                        {:else}
                          <span>📦</span>
                        {/if}
                      </td>
                      <td><strong>{p.name}</strong></td>
                      <td><span class="badge-tag">{p.category || 'Umum'}</span></td>
                      <td>{formatRupiah(p.price_cents)}</td>
                      <td><span class="badge-stock" class:empty={p.stock <= 0}>{p.stock} unit</span></td>
                      <td>
                        <button class="btn-action primary" onclick={() => openVariantModal(p)}>
                          ⚙️ Kelola Varian
                        </button>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          </div>

        {:else if subTab === 'orders'}
          <div class="section-panel">
            <h2>🛒 Pemrosesan Pesanan Pembeli</h2>
            {#if loading}
              <p>Memuat daftar pesanan...</p>
            {:else if orders.length === 0}
              <p class="empty-text">Belum ada pesanan dari pembeli.</p>
            {:else}
              <table class="table-custom">
                <thead>
                  <tr>
                    <th>ID Pesanan</th>
                    <th>Waktu</th>
                    <th>Total</th>
                    <th>Resi</th>
                    <th>Status</th>
                    <th>Aksi Fulfillment</th>
                  </tr>
                </thead>
                <tbody>
                  {#each orders as o (o.id)}
                    <tr>
                      <td><code>#{o.id.slice(0, 8)}</code></td>
                      <td>{new Date(o.created_at).toLocaleDateString('id-ID')}</td>
                      <td><strong>{formatRupiah(o.total_amount_cents)}</strong></td>
                      <td><small>{o.tracking_number || '-'}</small></td>
                      <td><span class="status-pill {o.status.toLowerCase()}">{o.status}</span></td>
                      <td>
                        {#if o.status === 'PAID'}
                          <button class="btn-action" onclick={() => updateOrderStatus(o.id, 'PROCESSING')}>Proses</button>
                        {:else if o.status === 'PROCESSING'}
                          <button class="btn-action primary" onclick={() => updateOrderStatus(o.id, 'SHIPPED')}>Kirim & Input Resi</button>
                        {:else}
                          <span class="done-mark">✓</span>
                        {/if}
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          </div>

        {:else if subTab === 'inventory'}
          <div class="section-panel">
            <h2>🏭 Monitoring Stok & Inventori</h2>
            <table class="table-custom">
              <thead>
                <tr>
                  <th>Nama Barang</th>
                  <th>Stok Tersedia</th>
                  <th>Kondisi</th>
                </tr>
              </thead>
              <tbody>
                {#each products as p (p.id)}
                  <tr>
                    <td><strong>{p.name}</strong></td>
                    <td>{p.stock} unit</td>
                    <td>
                      {#if p.stock <= 0}
                        <span class="badge-crit">Habis</span>
                      {:else if p.stock <= 5}
                        <span class="badge-warn">Kritis (&le;5)</span>
                      {:else}
                        <span class="badge-safe">Aman</span>
                      {/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>

        {:else if subTab === 'audit'}
          <div class="section-panel">
            <h2>📜 Riwayat Audit Trail Sistem</h2>
            {#if auditLogs.length === 0}
              <p class="empty-text">Belum ada log audit tercatat.</p>
            {:else}
              <table class="table-custom">
                <thead>
                  <tr>
                    <th>Waktu</th>
                    <th>Aksi</th>
                    <th>Tipe Resource</th>
                  </tr>
                </thead>
                <tbody>
                  {#each auditLogs as a (a.id)}
                    <tr>
                      <td><small>{new Date(a.created_at).toLocaleString('id-ID')}</small></td>
                      <td><code>{a.action}</code></td>
                      <td>{a.resource_type}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          </div>

        {:else if subTab === 'coupons'}
          <div class="section-panel">
            <div class="panel-top">
              <h2>🏷️ Manajemen Kupon & Kode Promo</h2>
              <button class="btn-add-prod" onclick={() => isAddCouponOpen = true}>+ Buat Kupon Baru</button>
            </div>

            {#if loading}
              <p>Memuat daftar kupon...</p>
            {:else if coupons.length === 0}
              <p class="empty-text">Belum ada kupon diskon. Klik tombol di atas untuk membuat kupon promo pertama!</p>
            {:else}
              <table class="table-custom">
                <thead>
                  <tr>
                    <th>Kode</th>
                    <th>Tipe & Diskon</th>
                    <th>Min. Belanja</th>
                    <th>Maks. Potongan</th>
                    <th>Pemakaian</th>
                    <th>Periode Berlaku</th>
                    <th>Status</th>
                    <th>Aksi</th>
                  </tr>
                </thead>
                <tbody>
                  {#each coupons as c (c.id)}
                    <tr>
                      <td><strong class="coupon-code-badge">{c.code}</strong></td>
                      <td>
                        {#if c.discount_type === 'PERCENTAGE'}
                          <span class="badge-tag discount-pct">{c.discount_value}% OFF</span>
                        {:else}
                          <span class="badge-tag discount-fix">{formatRupiah(c.discount_value)} OFF</span>
                        {/if}
                      </td>
                      <td>{c.min_order_amount > 0 ? formatRupiah(c.min_order_amount) : 'Tanpa Min.'}</td>
                      <td>{c.max_discount_amount ? formatRupiah(c.max_discount_amount) : '-'}</td>
                      <td>
                        <span class="usage-count">{c.usage_count}</span>
                        <span class="usage-limit">/ {c.usage_limit != null ? c.usage_limit : '∞'}</span>
                      </td>
                      <td>
                        <small>{new Date(c.start_date).toLocaleDateString('id-ID')} - {new Date(c.end_date).toLocaleDateString('id-ID')}</small>
                      </td>
                      <td>
                        <button
                          class="btn-toggle"
                          class:active={c.is_active}
                          onclick={() => handleToggleCoupon(c.id)}
                        >
                          {c.is_active ? '✓ Aktif' : '✕ Nonaktif'}
                        </button>
                      </td>
                      <td>
                        <button class="btn-var-del" onclick={() => handleDeleteCoupon(c.id, c.code)}>Hapus</button>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          </div>
        {/if}
      </section>
    </div>
  {/if}

  <!-- Add Product Modal -->
  {#if isAddProductOpen}
    <div class="modal-overlay" onclick={() => isAddProductOpen = false} role="button" tabindex="0" onkeydown={(e) => e.key === 'Escape' && (isAddProductOpen = false)}>
      <div class="modal-card" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
        <div class="modal-header">
          <h3>📦 Tambah Produk ke Katalog</h3>
          <button class="close-btn" onclick={() => isAddProductOpen = false}>&times;</button>
        </div>

        <form onsubmit={handleCreateProduct} class="prod-form">
          <label>
            Nama Produk:
            <input type="text" bind:value={newName} required placeholder="Contoh: Kemeja Flanel Premium" />
          </label>
          <label>
            Deskripsi:
            <textarea bind:value={newDesc} rows="2" placeholder="Detail spesifikasi produk"></textarea>
          </label>
          <div class="row-fields">
            <label>
              Harga (Rp):
              <input type="number" bind:value={newPrice} min="1000" step="1000" required />
            </label>
            <label>
              Stok Awal:
              <input type="number" bind:value={newStock} min="1" required />
            </label>
          </div>
          <label>
            Kategori:
            <input type="text" bind:value={newCategory} placeholder="Pakaian, Elektronik, Makanan, dll." />
          </label>
          <label>
            Cover Gambar Produk (Opsional):
            <input
              type="file"
              accept="image/*"
              onchange={(e: any) => selectedFile = e.target.files?.[0] || null}
            />
          </label>

          <div class="modal-actions">
            <button type="button" class="btn-cancel" onclick={() => isAddProductOpen = false}>Batal</button>
            <button type="submit" class="btn-save" disabled={isUploading}>
              {isUploading ? 'Menyimpan & Upload...' : 'Simpan Produk'}
            </button>
          </div>
        </form>
      </div>
    </div>
  {/if}

  <!-- Modal Kelola Varian Produk -->
  {#if selectedProductForVariants}
    <div class="modal-overlay" onclick={() => selectedProductForVariants = null} role="presentation">
      <div class="modal-card modal-variant-card" onclick={(e) => e.stopPropagation()} role="dialog">
        <div class="modal-header">
          <div>
            <h3>⚙️ Kelola Varian Produk</h3>
            <p class="var-prod-subtitle">{selectedProductForVariants.name} ({formatRupiah(selectedProductForVariants.price_cents)})</p>
          </div>
          <button class="close-btn" onclick={() => selectedProductForVariants = null}>&times;</button>
        </div>

        <!-- Daftar Varian Saat Ini -->
        <div class="var-list-section">
          <h4>Daftar Varian Aktif</h4>
          {#if variantsLoading}
            <p class="empty-text">Memuat varian...</p>
          {:else if variants.length === 0}
            <p class="empty-text">Belum ada varian untuk produk ini. Tambahkan di bawah.</p>
          {:else}
            <table class="table-custom var-table">
              <thead>
                <tr>
                  <th>Varian</th>
                  <th>Nilai</th>
                  <th>SKU</th>
                  <th>Harga Override</th>
                  <th>Stok</th>
                  <th>Aksi</th>
                </tr>
              </thead>
              <tbody>
                {#each variants as v (v.id)}
                  <tr>
                    <td><strong>{v.variant_name}</strong></td>
                    <td><span class="badge-tag">{v.variant_value}</span></td>
                    <td><code>{v.sku || '-'}</code></td>
                    <td>{v.price_override ? formatRupiah(v.price_override) : '(Standar)'}</td>
                    <td>{v.stock_quantity} unit</td>
                    <td>
                      <button class="btn-var-del" onclick={() => handleDeleteVariant(v.id)}>Hapus</button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {/if}
        </div>

        <!-- Form Tambah Varian Baru -->
        <form class="var-add-form" onsubmit={(e) => { e.preventDefault(); handleAddVariant(); }}>
          <h4>+ Tambah Varian Baru</h4>
          <div class="row-fields">
            <label>
              Nama Varian
              <input type="text" bind:value={varName} placeholder="Ukuran, Warna, dll" required />
            </label>
            <label>
              Nilai Varian
              <input type="text" bind:value={varValue} placeholder="XL, Merah, 256GB" required />
            </label>
          </div>
          <div class="row-fields">
            <label>
              SKU Varian (Opsional)
              <input type="text" bind:value={varSku} placeholder="SKU-PROD-VAR" />
            </label>
            <label>
              Harga Khusus (Rp)
              <input type="number" bind:value={varPriceOverride} placeholder="Kosongkan jika harga standar" min="0" />
            </label>
            <label>
              Stok Varian
              <input type="number" bind:value={varStock} min="0" required />
            </label>
          </div>
          <div class="modal-actions">
            <button type="button" class="btn-cancel" onclick={() => selectedProductForVariants = null}>Tutup</button>
            <button type="submit" class="btn-save" disabled={varSaving}>
              {varSaving ? 'Menyimpan...' : '+ Tambah Varian'}
            </button>
          </div>
        </form>
      </div>
    </div>
  {/if}

  <!-- Modal Buat Kupon Diskon -->
  {#if isAddCouponOpen}
    <div class="modal-overlay" onclick={() => isAddCouponOpen = false} role="button" tabindex="0" onkeydown={(e) => e.key === 'Escape' && (isAddCouponOpen = false)}>
      <div class="modal-card" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
        <div class="modal-header">
          <h3>🏷️ Buat Kupon Diskon Baru</h3>
          <button class="close-btn" onclick={() => isAddCouponOpen = false}>&times;</button>
        </div>

        <form onsubmit={handleCreateCoupon} class="prod-form">
          <div class="row-fields">
            <label>
              Kode Kupon (Otomatis Kapital):
              <input type="text" bind:value={newCouponCode} required placeholder="Contoh: HEMAT20" style="text-transform: uppercase;" />
            </label>
            <label>
              Tipe Diskon:
              <select bind:value={newCouponDiscountType} class="select-custom">
                <option value="PERCENTAGE">Persentase (%)</option>
                <option value="FIXED_AMOUNT">Nominal Tetap (Rp)</option>
              </select>
            </label>
          </div>

          <div class="row-fields">
            <label>
              Nilai Diskon:
              <input
                type="number"
                bind:value={newCouponDiscountVal}
                min="1"
                max={newCouponDiscountType === 'PERCENTAGE' ? 100 : undefined}
                required
              />
            </label>
            {#if newCouponDiscountType === 'PERCENTAGE'}
              <label>
                Maksimal Potongan (Rp, Opsional):
                <input type="number" bind:value={newCouponMaxCap} placeholder="Kosongkan jika tanpa batas" min="0" />
              </label>
            {/if}
          </div>

          <div class="row-fields">
            <label>
              Min. Nilai Pesanan (Rp):
              <input type="number" bind:value={newCouponMinOrder} min="0" placeholder="0 = tanpa minimum" />
            </label>
            <label>
              Batas Total Pemakaian (Opsional):
              <input type="number" bind:value={newCouponLimit} min="1" placeholder="Kosongkan = tanpa batas" />
            </label>
          </div>

          <div class="row-fields">
            <label>
              Tanggal Mulai Berlaku:
              <input type="datetime-local" bind:value={newCouponStartDate} required />
            </label>
            <label>
              Tanggal Kedaluwarsa:
              <input type="datetime-local" bind:value={newCouponEndDate} required />
            </label>
          </div>

          <div class="modal-actions">
            <button type="button" class="btn-cancel" onclick={() => isAddCouponOpen = false}>Batal</button>
            <button type="submit" class="btn-save" disabled={isCreatingCoupon}>
              {isCreatingCoupon ? 'Membuat Kupon...' : 'Simpan Kupon 🚀'}
            </button>
          </div>
        </form>
      </div>
    </div>
  {/if}
</div>

<style>
  .admin-wrapper { width: 100%; min-height: 80vh; }
  .admin-login-card {
    max-width: 400px; margin: 3rem auto; background: #0f172a;
    border: 1px solid #334155; border-radius: 12px; padding: 2rem;
    box-shadow: 0 10px 30px rgba(0,0,0,0.5); text-align: center;
  }
  .lock-icon { font-size: 2.5rem; }
  .login-header h2 { margin: 0.5rem 0 0.25rem; color: #38bdf8; }
  .login-header p { font-size: 0.85rem; color: #94a3b8; margin-bottom: 1.5rem; }
  .login-form { display: flex; flex-direction: column; gap: 1rem; text-align: left; }
  .login-form label { font-size: 0.85rem; color: #cbd5e1; display: flex; flex-direction: column; gap: 0.3rem; }
  .login-form input {
    background: #1e293b; border: 1px solid #334155; border-radius: 6px;
    padding: 0.65rem; color: #fff;
  }
  .btn-submit {
    background: #0284c7; color: #fff; border: none; padding: 0.75rem;
    border-radius: 6px; font-weight: bold; cursor: pointer; margin-top: 0.5rem;
  }

  /* Dashboard Layout */
  .admin-container { display: flex; gap: 1.5rem; min-height: 75vh; }
  .sidebar {
    width: 220px; background: #0f172a; border: 1px solid #1e293b;
    border-radius: 12px; padding: 1.25rem; display: flex; flex-direction: column; gap: 1rem;
  }
  .side-header h3 { margin: 0 0 0.25rem; color: #38bdf8; font-size: 1.15rem; }
  .admin-name { font-size: 0.8rem; color: #94a3b8; }
  .side-nav { display: flex; flex-direction: column; gap: 0.4rem; flex: 1; }
  .side-nav button {
    background: transparent; border: none; color: #94a3b8; text-align: left;
    padding: 0.65rem 0.8rem; border-radius: 6px; cursor: pointer; font-size: 0.9rem;
    transition: all 0.2s;
  }
  .side-nav button:hover { background: #1e293b; color: #fff; }
  .side-nav button.active { background: #0284c7; color: #fff; font-weight: bold; }
  .btn-logout {
    background: #7f1d1d; color: #fecaca; border: none; padding: 0.5rem;
    border-radius: 6px; cursor: pointer; font-size: 0.85rem;
  }

  .admin-main { flex: 1; background: #0f172a; border: 1px solid #1e293b; border-radius: 12px; padding: 1.5rem; }
  .kpi-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 1rem; margin-top: 1.25rem; }
  .kpi-card {
    background: #1e293b; border: 1px solid #334155; border-radius: 10px;
    padding: 1.25rem; display: flex; flex-direction: column;
  }
  .kpi-label { font-size: 0.85rem; color: #94a3b8; }
  .kpi-val { font-size: 1.8rem; color: #38bdf8; margin: 0.5rem 0; font-weight: bold; }
  .kpi-hint { font-size: 0.75rem; color: #64748b; }

  .panel-top { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
  .btn-add-prod {
    background: #059669; color: #fff; border: none; padding: 0.55rem 1rem;
    border-radius: 6px; font-weight: bold; cursor: pointer;
  }

  /* Table */
  .table-custom { width: 100%; border-collapse: collapse; margin-top: 1rem; }
  .table-custom th, .table-custom td {
    padding: 0.75rem 1rem; text-align: left; border-bottom: 1px solid #1e293b; font-size: 0.9rem;
  }
  .table-custom th { background: #1e293b; color: #94a3b8; font-size: 0.8rem; }
  .td-img { width: 44px; height: 44px; text-align: center; }
  .td-img img { width: 40px; height: 40px; object-fit: cover; border-radius: 4px; }
  .badge-tag { background: #334155; padding: 0.2rem 0.5rem; border-radius: 4px; font-size: 0.75rem; }
  .badge-stock { color: #10b981; font-weight: bold; }
  .badge-stock.empty { color: #ef4444; }
  .badge-crit { background: #dc2626; color: #fff; padding: 0.2rem 0.4rem; border-radius: 4px; font-size: 0.75rem; }
  .badge-warn { background: #d97706; color: #fff; padding: 0.2rem 0.4rem; border-radius: 4px; font-size: 0.75rem; }
  .badge-safe { background: #059669; color: #fff; padding: 0.2rem 0.4rem; border-radius: 4px; font-size: 0.75rem; }
  .status-pill { padding: 0.2rem 0.5rem; border-radius: 4px; font-size: 0.75rem; font-weight: bold; }
  .status-pill.paid { background: #0284c7; }
  .status-pill.processing { background: #d97706; }
  .status-pill.shipped { background: #7c3aed; }
  .btn-action {
    background: #334155; color: #fff; border: none; padding: 0.35rem 0.7rem;
    border-radius: 4px; cursor: pointer; font-size: 0.8rem;
  }
  .btn-action.primary { background: #0284c7; font-weight: bold; }
  .done-mark { color: #10b981; font-weight: bold; }
  .empty-text { color: #94a3b8; text-align: center; padding: 2rem 0; }

  /* Modal */
  .modal-overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.7);
    display: flex; align-items: center; justify-content: center; z-index: 1000;
  }
  .modal-card {
    background: #0f172a; border: 1px solid #334155; border-radius: 12px;
    width: 90%; max-width: 480px; padding: 1.5rem; color: #f8fafc;
  }
  .modal-header { display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid #334155; padding-bottom: 0.5rem; }
  .close-btn { background: none; border: none; font-size: 1.4rem; color: #94a3b8; cursor: pointer; }
  .prod-form { display: flex; flex-direction: column; gap: 0.85rem; margin-top: 1rem; }
  .prod-form label { font-size: 0.85rem; color: #cbd5e1; display: flex; flex-direction: column; gap: 0.25rem; }
  .prod-form input, .prod-form textarea {
    background: #1e293b; border: 1px solid #334155; border-radius: 6px;
    padding: 0.6rem; color: #fff; font-family: inherit;
  }
  .row-fields { display: flex; gap: 1rem; }
  .row-fields label { flex: 1; }
  .modal-actions { display: flex; justify-content: flex-end; gap: 0.75rem; margin-top: 0.75rem; }
  .btn-cancel { background: #334155; color: #fff; border: none; padding: 0.6rem 1.2rem; border-radius: 6px; cursor: pointer; }
  .btn-save { background: #059669; color: #fff; border: none; padding: 0.6rem 1.2rem; border-radius: 6px; font-weight: bold; cursor: pointer; }

  /* Variant Management Modal */
  .modal-variant-card { max-width: 680px; }
  .var-prod-subtitle { font-size: 0.85rem; color: #38bdf8; margin: 0.25rem 0 0; }
  .var-list-section { margin-top: 1rem; max-height: 220px; overflow-y: auto; }
  .var-list-section h4 { margin: 0 0 0.5rem; font-size: 0.95rem; color: #94a3b8; }
  .var-table th, .var-table td { padding: 0.5rem 0.75rem; font-size: 0.85rem; }
  .btn-var-del { background: #7f1d1d; color: #fecaca; border: none; padding: 0.25rem 0.5rem; border-radius: 4px; cursor: pointer; font-size: 0.75rem; }
  .btn-var-del:hover { background: #991b1b; }
  .var-add-form { margin-top: 1.5rem; border-top: 1px solid #334155; padding-top: 1rem; display: flex; flex-direction: column; gap: 0.75rem; }
  .var-add-form h4 { margin: 0 0 0.25rem; font-size: 0.95rem; color: #38bdf8; }
  .var-add-form label { font-size: 0.8rem; color: #cbd5e1; display: flex; flex-direction: column; gap: 0.2rem; }
  .var-add-form input { background: #1e293b; border: 1px solid #334155; border-radius: 6px; padding: 0.55rem; color: #fff; }

  /* Coupon Specific Styles */
  .coupon-code-badge {
    background: #1e3a8a; color: #93c5fd; padding: 0.2rem 0.5rem;
    border-radius: 4px; font-family: monospace; font-size: 0.85rem; letter-spacing: 0.5px;
  }
  .discount-pct { background: #065f46; color: #6ee7b7; font-weight: bold; }
  .discount-fix { background: #7c2d12; color: #fdba74; font-weight: bold; }
  .usage-count { color: #38bdf8; font-weight: bold; }
  .usage-limit { color: #64748b; font-size: 0.8rem; }
  .btn-toggle {
    border: none; padding: 0.25rem 0.6rem; border-radius: 4px;
    font-size: 0.75rem; font-weight: bold; cursor: pointer;
    background: #334155; color: #94a3b8; transition: all 0.2s;
  }
  .btn-toggle.active { background: #059669; color: #ecfdf5; }
  .select-custom {
    background: #1e293b; border: 1px solid #334155; border-radius: 6px;
    padding: 0.6rem; color: #fff; font-family: inherit;
  }
</style>
