# Issue 16: Migrasi Admin Hub ke Svelte SPA (Dashboard, Inventori, Order & Analytics)

> **Prioritas:** 🟡 HIGH  
> **Estimasi:** 4-5 hari  
> **Kesulitan:** ⭐⭐⭐⭐ Menengah-Lanjut  
> **Prerequisite:** Issue 11 & Issue 12

---

## 📋 Deskripsi

Antarmuka pengelola toko (Admin Hub) saat ini diakses via `/admin` menggunakan file statis monolithic `index.html` dan 8 script JavaScript terpisah di folder `crates/web/static/admin/`.  
Di Issue ini, kita akan:
1. Membangun **Admin Hub SPA berbasis Svelte 5** dengan otorisasi khusus (Role `Admin` / `Seller`).
2. Membuat layout navigasi sidebar modern (Overview, Produk & Varian, Inventori/Stok, Pesanan Masuk, Pengguna, dan Audit Log).
3. Mengintegrasikan fitur unggah file gambar produk (`multipart/form-data`) ke endpoint `/v1/upload`.
4. Mengelola status fulfillment pesanan pembeli (proses pesanan, input nomor resi pengiriman, dan pembatalan).

---

## 🎯 Acceptance Criteria

- [ ] Token JWT Admin disimpan terpisah dari token Buyer (`program1_admin_token`).
- [ ] Route guard mencegah akses ke panel admin jika user belum login atau tidak memiliki peran `Admin` / `Seller`.
- [ ] Form tambah/edit produk mendukung upload gambar langsung ke backend via `POST /v1/upload`.
- [ ] Tabel Inventori memungkinkan penyesuaian stok instan (`quick stock adjust`) dengan validasi angka positif.
- [ ] Modul Pesanan (`AdminOrders.svelte`) memungkinkan admin memperbarui status pesanan dari `PAID` ke `PROCESSING` lalu `SHIPPED` dengan nomor resi.
- [ ] Dashboard Analitik menampilkan metrik ringkasan penjualan dan grafik tren.

---

## 📐 Langkah-Langkah Pengerjaan

### Langkah 1: Buat Store Autentikasi Khusus Admin

**File:** `frontend/src/lib/stores/adminAuth.svelte.ts`

```typescript
import { apiFetch } from '$lib/api/client';
import { toast } from './toast.svelte';

const ADMIN_TOKEN_KEY = 'program1_admin_token';

interface AdminUser {
  id: string;
  username: string;
  role: 'Admin' | 'Seller';
}

class AdminAuthState {
  token = $state<string | null>(localStorage.getItem(ADMIN_TOKEN_KEY));
  user = $state<AdminUser | null>(null);
  isLoading = $state(false);

  constructor() {
    if (this.token) {
      this.checkAuth();
    }
  }

  async checkAuth() {
    try {
      this.user = await apiFetch<AdminUser>('/auth/me', {
        headers: { Authorization: `Bearer ${this.token}` },
      });
    } catch {
      this.logout();
    }
  }

  async login(username: string, password: string): Promise<boolean> {
    this.isLoading = true;
    try {
      const res = await apiFetch<{ token: string; user: AdminUser }>('/auth/login', {
        method: 'POST',
        body: JSON.stringify({ username, password }),
      });
      this.token = res.token;
      this.user = res.user;
      localStorage.setItem(ADMIN_TOKEN_KEY, res.token);
      toast.success(`Selamat datang di Admin Hub, ${res.user.username}!`);
      return true;
    } catch (err: any) {
      toast.error(err.message || 'Login Admin gagal');
      return false;
    } finally {
      this.isLoading = false;
    }
  }

  logout() {
    this.token = null;
    this.user = null;
    localStorage.removeItem(ADMIN_TOKEN_KEY);
    toast.info('Keluar dari Admin Hub.');
  }
}

export const adminAuth = new AdminAuthState();
```

---

### Langkah 2: Helper Upload Gambar Produk

**File:** `frontend/src/lib/api/upload.ts`

```typescript
export async function uploadProductImage(file: File, token: string): Promise<string> {
  const formData = new FormData();
  formData.append('image', file);

  const res = await fetch('/v1/upload', {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${token}`,
    },
    body: formData,
  });

  if (!res.ok) {
    const err = await res.json().catch(() => ({ message: 'Upload gagal' }));
    throw new Error(err.message || 'Gagal mengunggah gambar produk');
  }

  const data = await res.json();
  // Backend mengembalikan { url: "/uploads/filename.webp" }
  return data.url;
}
```

---

### Langkah 3: Buat Layout Admin Sidebar `AdminLayout.svelte`

**File:** `frontend/src/lib/components/admin/AdminLayout.svelte`

```svelte
<script lang="ts">
  import { adminAuth } from '$lib/stores/adminAuth.svelte';

  interface Props {
    activeTab: string;
    onTabChange: (tab: string) => void;
    children: any;
  }

  let { activeTab, onTabChange, children }: Props = $props();

  const menuItems = [
    { id: 'analytics', label: '📊 Analitik & KPI' },
    { id: 'catalog', label: '📦 Katalog Produk' },
    { id: 'inventory', label: '🏭 Inventori & Stok' },
    { id: 'orders', label: '🛒 Pesanan Masuk' },
    { id: 'audit', label: '📜 Audit Logs' },
  ];
</script>

<div class="admin-shell">
  <aside class="sidebar">
    <div class="sidebar-header">
      <h2>Program1 Hub</h2>
      <span class="role-tag">{adminAuth.user?.role || 'Admin'}</span>
    </div>

    <nav class="sidebar-nav">
      {#each menuItems as item}
        <button
          class:active={activeTab === item.id}
          onclick={() => onTabChange(item.id)}
        >
          {item.label}
        </button>
      {/each}
    </nav>

    <div class="sidebar-footer">
      <small>Logged as: <strong>{adminAuth.user?.username}</strong></small>
      <button onclick={() => adminAuth.logout()} class="btn-logout">Keluar</button>
    </div>
  </aside>

  <main class="main-content">
    {@render children()}
  </main>
</div>

<style>
  .admin-shell { display: flex; min-height: 100vh; background: #0f172a; color: #f8fafc; }
  .sidebar {
    width: 240px; background: #1e293b; border-right: 1px solid #334155;
    display: flex; flex-direction: column; padding: 1rem;
  }
  .sidebar-header { margin-bottom: 1.5rem; border-bottom: 1px solid #334155; padding-bottom: 1rem; }
  .sidebar-header h2 { margin: 0; font-size: 1.25rem; color: #38bdf8; }
  .role-tag {
    font-size: 0.75rem; background: #0284c7; color: #fff;
    padding: 0.15rem 0.5rem; border-radius: 4px; font-weight: bold;
  }
  .sidebar-nav { display: flex; flex-direction: column; gap: 0.4rem; flex: 1; }
  .sidebar-nav button {
    background: none; border: none; color: #94a3b8; text-align: left;
    padding: 0.75rem 1rem; border-radius: 6px; font-size: 0.95rem;
    cursor: pointer; transition: all 0.2s;
  }
  .sidebar-nav button:hover { background: #334155; color: #f8fafc; }
  .sidebar-nav button.active { background: #0284c7; color: #fff; font-weight: 600; }
  .sidebar-footer { border-top: 1px solid #334155; padding-top: 1rem; display: flex; flex-direction: column; gap: 0.5rem; }
  .btn-logout { background: #dc2626; color: #fff; border: none; padding: 0.5rem; border-radius: 4px; cursor: pointer; }
  .main-content { flex: 1; padding: 2rem; overflow-y: auto; }
</style>
```

---

### Langkah 4: Buat Modul Manajemen Pesanan `AdminOrders.svelte`

**File:** `frontend/src/lib/components/admin/AdminOrders.svelte`

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { apiFetch } from '$lib/api/client';
  import { adminAuth } from '$lib/stores/adminAuth.svelte';
  import { toast } from '$lib/stores/toast.svelte';
  import { formatRupiah } from '$lib/utils/currency';

  interface OrderRecord {
    id: string;
    buyer_name: string;
    total_amount_cents: number;
    status: string;
    created_at: string;
    tracking_number?: string;
  }

  let orders = $state<OrderRecord[]>([]);
  let loading = $state(true);

  async function loadOrders() {
    loading = true;
    try {
      orders = await apiFetch<OrderRecord[]>('/orders', {
        headers: { Authorization: `Bearer ${adminAuth.token}` },
      });
    } catch (err: any) {
      toast.error(err.message || 'Gagal memuat pesanan');
    } finally {
      loading = false;
    }
  }

  async function updateStatus(orderId: string, nextStatus: string) {
    let trackingNumber: string | null = null;
    if (nextStatus === 'SHIPPED') {
      trackingNumber = prompt('Masukkan Nomor Resi Pengiriman (Awb / Tracking):');
      if (!trackingNumber) return;
    }

    try {
      await apiFetch(`/orders/${orderId}/status`, {
        method: 'PUT',
        headers: { Authorization: `Bearer ${adminAuth.token}` },
        body: JSON.stringify({ status: nextStatus, tracking_number: trackingNumber }),
      });
      toast.success(`Status pesanan #${orderId.slice(0, 8)} berhasil diubah ke ${nextStatus}`);
      await loadOrders();
    } catch (err: any) {
      toast.error(err.message || 'Gagal mengubah status pesanan');
    }
  }

  onMount(() => {
    loadOrders();
  });
</script>

<div class="panel">
  <h2>🛒 Manajemen Pesanan Pembeli</h2>

  {#if loading}
    <p>Memuat data pesanan...</p>
  {:else}
    <table class="data-table">
      <thead>
        <tr>
          <th>ID Pesanan</th>
          <th>Tanggal</th>
          <th>Total</th>
          <th>Status</th>
          <th>Aksi</th>
        </tr>
      </thead>
      <tbody>
        {#each orders as o (o.id)}
          <tr>
            <td><code>#{o.id.slice(0, 8)}</code></td>
            <td>{new Date(o.created_at).toLocaleDateString('id-ID')}</td>
            <td><strong>{formatRupiah(o.total_amount_cents)}</strong></td>
            <td><span class="badge {o.status.toLowerCase()}">{o.status}</span></td>
            <td>
              {#if o.status === 'PAID'}
                <button onclick={() => updateStatus(o.id, 'PROCESSING')} class="btn-action">
                  Proses
                </button>
              {:else if o.status === 'PROCESSING'}
                <button onclick={() => updateStatus(o.id, 'SHIPPED')} class="btn-action primary">
                  Kirim & Input Resi
                </button>
              {:else}
                <span class="text-muted">-</span>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .panel { background: #1e293b; padding: 1.5rem; border-radius: 10px; border: 1px solid #334155; }
  .data-table { width: 100%; border-collapse: collapse; margin-top: 1rem; }
  .data-table th, .data-table td { padding: 0.75rem 1rem; text-align: left; border-bottom: 1px solid #334155; }
  .data-table th { background: #0f172a; color: #94a3b8; font-size: 0.85rem; }
  .badge { padding: 0.2rem 0.5rem; border-radius: 4px; font-size: 0.75rem; font-weight: bold; }
  .badge.paid { background: #0284c7; }
  .badge.processing { background: #d97706; }
  .badge.shipped { background: #7c3aed; }
  .badge.delivered { background: #059669; }
  .btn-action {
    background: #334155; color: #fff; border: none; padding: 0.4rem 0.8rem;
    border-radius: 4px; cursor: pointer; font-size: 0.85rem;
  }
  .btn-action.primary { background: #0284c7; }
</style>
```

---

## 🧪 Pengujian & Verifikasi

1. Buka halaman admin: `http://localhost:5173/#/admin` (atau route `/admin`).
2. Login menggunakan akun admin (kredensial yang di-seed di backend).
3. Pastikan layout sidebar tampil dengan seluruh tab navigasi.
4. Buka tab **Manajemen Pesanan**, coba ubah pesanan berstatus `PAID` menjadi `PROCESSING`, lalu `SHIPPED` dengan memasukkan nomor resi.
5. Verifikasi di dashboard pembeli bahwa status pesanan dan nomor resi ter-update secara akurat.
6. Jalankan tes Rust: `cargo test --workspace` untuk menjamin integritas kontrak.

---

## ⚠️ Hal Penting & Gotchas

- **Header Authorization:** Endpoint admin (`/orders`, `/catalog/manage`, `/inventory/*`) mewajibkan token JWT admin. Jangan mencampur token buyer ke dalam header admin.
- **Multipart Upload:** Saat mengunggah file lewat `FormData`, **JANGAN** set header `Content-Type: application/json`. Biarkan browser mengatur boundary multipart secara otomatis.
