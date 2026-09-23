<script lang="ts">
  import { onMount } from 'svelte';
  import { adminAuth } from './adminAuth.svelte';
  import { toast } from './toast.svelte';
  import { i18n } from './i18n.svelte';
  import NotificationBell from './NotificationBell.svelte';
  import type { Product, Coupon, ProductReview, Category } from './types';
  import { fetchAdmin } from './admin/adminApi';

  import AdminKpi from './admin/AdminKpi.svelte';
  import AdminReports from './admin/AdminReports.svelte';
  import AdminCatalog from './admin/AdminCatalog.svelte';
  import AdminCategories from './admin/AdminCategories.svelte';
  import AdminInventory from './admin/AdminInventory.svelte';
  import AdminOrders from './admin/AdminOrders.svelte';
  import AdminCoupons from './admin/AdminCoupons.svelte';
  import AdminReviews from './admin/AdminReviews.svelte';
  import AdminReturns from './admin/AdminReturns.svelte';
  import AdminBackup from './admin/AdminBackup.svelte';
  import AdminSuppliers from './admin/AdminSuppliers.svelte';
  import AdminAudit from './admin/AdminAudit.svelte';

  interface AdminOrder {
    id: string;
    total_amount_cents: number;
    status: string;
    created_at: string;
    tracking_number?: string | null;
  }

  interface AuditLogRecord {
    id: string;
    actor_username?: string;
    action: string;
    resource_type: string;
    created_at: string;
  }

  // Active sub-tab
  let subTab = $state<'kpi' | 'reports' | 'catalog' | 'categories' | 'inventory' | 'orders' | 'audit' | 'coupons' | 'reviews' | 'returns' | 'backup' | 'suppliers'>('kpi');

  // Login form state
  let loginUser = $state('admin');
  let loginPass = $state('admin123');

  // Data states
  let products = $state<Product[]>([]);
  let orders = $state<AdminOrder[]>([]);
  let auditLogs = $state<AuditLogRecord[]>([]);
  let coupons = $state<Coupon[]>([]);
  let categories = $state<Category[]>([]);
  let adminReviews = $state<ProductReview[]>([]);
  let loading = $state(false);

  function handleAdminNotificationClick(item: any) {
    if (item.notification_type === 'return' || item.reference_type === 'return') {
      subTab = 'returns';
    } else if (item.notification_type === 'new_order' || item.reference_type === 'order') {
      subTab = 'orders';
    } else if (item.notification_type === 'low_stock' || item.reference_type === 'product') {
      subTab = 'inventory';
    }
  }

  async function handleLogin(e: Event) {
    e.preventDefault();
    try {
      await adminAuth.login(loginUser, loginPass);
      toast.success('Login admin berhasil!');
      await loadData();
    } catch (err: any) {
      toast.error(err.message || 'Login admin gagal');
    }
  }

  async function loadData() {
    if (!adminAuth.token) return;
    loading = true;
    try {
      const [catRes, ordRes, audRes, coupRes, categRes, revRes] = await Promise.all([
        fetchAdmin<{ items: Product[] }>('/catalog?page=1&page_size=50').catch(() => ({ items: [] })),
        fetchAdmin<AdminOrder[]>('/orders').catch(() => []),
        fetchAdmin<AuditLogRecord[]>('/audit/logs').catch(() => []),
        fetchAdmin<Coupon[]>('/api/v1/admin/coupons').catch(() => []),
        fetchAdmin<Category[]>('/api/v1/categories').catch(() => []),
        fetchAdmin<{ items: ProductReview[] }>('/api/v1/admin/reviews?page=1&page_size=20').catch(() => ({ items: [] })),
      ]);

      products = catRes.items || [];
      orders = ordRes || [];
      auditLogs = audRes || [];
      coupons = coupRes || [];
      categories = categRes || [];
      adminReviews = revRes.items || [];
    } catch (err: any) {
      toast.error(err.message || 'Gagal memuat data admin');
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (adminAuth.token) {
      loadData();
    }
  });

  onMount(() => {
    if (adminAuth.token) {
      loadData();
    }
  });
</script>

<div class="admin-wrapper">
  {#if !adminAuth.token}
    <!-- Login Admin Screen -->
    <div class="admin-login-card">
      <div class="lock-icon">🔐</div>
      <div class="login-header">
        <h2>Panel Kontrol Admin</h2>
        <p>Masuk dengan kredensial administrator Anda</p>
      </div>

      <form onsubmit={handleLogin} class="login-form">
        <label>
          Username:
          <input type="text" bind:value={loginUser} required />
        </label>
        <label>
          Password:
          <input type="password" bind:value={loginPass} required />
        </label>
        <button type="submit" class="btn-submit" disabled={adminAuth.loading}>
          {adminAuth.loading ? 'Memverifikasi...' : 'Masuk ke Admin Hub'}
        </button>
      </form>
    </div>
  {:else}
    <!-- Admin Dashboard Main Layout -->
    <div class="admin-container">
      <!-- Sidebar Navigation -->
      <aside class="sidebar no-print">
        <div class="side-header">
          <div class="side-brand-row">
            <h3>⚙️ Admin Hub</h3>
            <div class="header-actions-right">
              <div class="admin-lang-pills">
                <button
                  class="btn-admin-lang"
                  class:active={i18n.current === 'id'}
                  onclick={() => i18n.setLanguage('id')}
                  title="Bahasa Indonesia"
                >
                  ID
                </button>
                <button
                  class="btn-admin-lang"
                  class:active={i18n.current === 'en'}
                  onclick={() => i18n.setLanguage('en')}
                  title="English"
                >
                  EN
                </button>
              </div>
              <NotificationBell role="admin" onNotificationClick={handleAdminNotificationClick} />
            </div>
          </div>
          <span class="admin-name">Halo, <strong>{adminAuth.username}</strong></span>
        </div>

        <nav class="side-nav">
          <button class:active={subTab === 'kpi'} onclick={() => subTab = 'kpi'}>📊 {i18n.t('admin.kpi_summary', 'Ringkasan KPI')}</button>
          <button class:active={subTab === 'reports'} onclick={() => subTab = 'reports'}>📑 {i18n.t('admin.sales_reports', 'Laporan Penjualan')}</button>
          <button class:active={subTab === 'catalog'} onclick={() => subTab = 'catalog'}>📦 {i18n.t('admin.catalog_mgmt', 'Katalog Produk')} ({products.length})</button>
          <button class:active={subTab === 'categories'} onclick={() => subTab = 'categories'}>🏷️ {i18n.t('admin.category_mgmt', 'Kategori Produk')} ({categories.length})</button>
          <button class:active={subTab === 'inventory'} onclick={() => subTab = 'inventory'}>🏭 {i18n.t('admin.inventory_mgmt', 'Stok & Inventori')}</button>
          <button class:active={subTab === 'orders'} onclick={() => subTab = 'orders'}>🛒 {i18n.t('admin.orders_mgmt', 'Pesanan Masuk')} ({orders.length})</button>
          <button class:active={subTab === 'coupons'} onclick={() => subTab = 'coupons'}>🏷️ {i18n.t('admin.coupons_mgmt', 'Kupon Diskon')} ({coupons.length})</button>
          <button class:active={subTab === 'reviews'} onclick={() => subTab = 'reviews'}>⭐ {i18n.t('admin.reviews_mgmt', 'Moderasi Ulasan')}</button>
          <button class:active={subTab === 'returns'} onclick={() => subTab = 'returns'}>🔄 {i18n.t('admin.returns_mgmt', 'Retur & Refund')}</button>
          <button class:active={subTab === 'backup'} onclick={() => subTab = 'backup'}>💾 {i18n.t('admin.database_backup', 'Database & Backup')}</button>
          <button class:active={subTab === 'suppliers'} onclick={() => subTab = 'suppliers'}>🏢 {i18n.t('admin.suppliers_mgmt', 'Supplier & Restock PO')}</button>
          <button class:active={subTab === 'audit'} onclick={() => subTab = 'audit'}>📜 {i18n.t('admin.audit_logs', 'Audit Logs')}</button>
        </nav>

        <button class="btn-logout" onclick={() => adminAuth.logout()}>🚪 Keluar Sesi Admin</button>
      </aside>

      <!-- Main Content Panel -->
      <section class="admin-main">
        {#if subTab === 'kpi'}
          <AdminKpi
            {products}
            {orders}
            {coupons}
            {adminReviews}
            onOpenReports={() => subTab = 'reports'}
          />
        {:else if subTab === 'reports'}
          <AdminReports />
        {:else if subTab === 'catalog'}
          <AdminCatalog
            {products}
            {categories}
            {loading}
            onRefresh={loadData}
          />
        {:else if subTab === 'categories'}
          <AdminCategories
            {categories}
            {loading}
            onRefresh={loadData}
          />
        {:else if subTab === 'inventory'}
          <AdminInventory {products} />
        {:else if subTab === 'orders'}
          <AdminOrders
            {orders}
            {loading}
            onRefresh={loadData}
          />
        {:else if subTab === 'coupons'}
          <AdminCoupons
            {coupons}
            {loading}
            onRefresh={loadData}
          />
        {:else if subTab === 'reviews'}
          <AdminReviews {products} />
        {:else if subTab === 'returns'}
          <AdminReturns />
        {:else if subTab === 'backup'}
          <AdminBackup />
        {:else if subTab === 'suppliers'}
          <AdminSuppliers {products} />
        {:else if subTab === 'audit'}
          <AdminAudit {auditLogs} />
        {/if}
      </section>
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
  .side-header h3 { margin: 0; color: #38bdf8; font-size: 1.15rem; }
  .side-brand-row { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.35rem; }
  .header-actions-right { display: flex; align-items: center; gap: 0.5rem; }
  .admin-lang-pills { display: flex; gap: 2px; background: #1e293b; border-radius: 12px; padding: 2px; border: 1px solid #334155; }
  .btn-admin-lang {
    background: transparent; border: none; color: #94a3b8; font-size: 0.7rem; font-weight: 700;
    padding: 0.15rem 0.45rem; border-radius: 10px; cursor: pointer; transition: all 0.2s;
  }
  .btn-admin-lang:hover { color: #fff; }
  .btn-admin-lang.active { background: #0284c7; color: #fff; }
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
</style>
