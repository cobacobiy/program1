<script lang="ts">
  import { onMount } from 'svelte';
  import type { Product } from '../types';
  import { formatRupiah } from '../currency';
  import { toast } from '../toast.svelte';
  import { fetchAdmin } from './adminApi';

  interface SupplierItem {
    id: string;
    name: string;
    contact_person?: string | null;
    phone?: string | null;
    email?: string | null;
    address?: string | null;
    is_active: boolean;
    created_at: string;
  }

  interface PoItemDto {
    id: string;
    product_id: string;
    quantity: number;
    unit_cost_cents: number;
    total_cost_cents: number;
  }

  interface PoRecord {
    id: string;
    po_number: string;
    supplier_id: string;
    supplier_name?: string | null;
    status: string;
    notes?: string | null;
    total_cost_cents: number;
    ordered_at: string;
    received_at?: string | null;
    items: PoItemDto[];
  }

  interface Props {
    products: Product[];
  }

  let { products }: Props = $props();

  let suppliersList = $state<SupplierItem[]>([]);
  let poList = $state<PoRecord[]>([]);
  let supplierLoading = $state(false);
  let isAddSupplierOpen = $state(false);
  let isAddPoOpen = $state(false);
  let newSupName = $state('');
  let newSupContact = $state('');
  let newSupPhone = $state('');
  let newSupEmail = $state('');
  let newSupAddress = $state('');
  let newPoSupplierId = $state('');
  let newPoProductId = $state('');
  let newPoQuantity = $state(50);
  let newPoUnitCost = $state(50000);
  let newPoNotes = $state('');

  export async function loadSuppliersAndPo() {
    supplierLoading = true;
    try {
      const [sup, po] = await Promise.all([
        fetchAdmin<SupplierItem[]>('/api/v1/admin/suppliers').catch(() => []),
        fetchAdmin<PoRecord[]>('/api/v1/admin/purchase-orders').catch(() => [])
      ]);
      suppliersList = sup || [];
      poList = po || [];
    } catch (err: any) {
      toast.error(err.message || 'Gagal memuat supplier & PO');
    } finally {
      supplierLoading = false;
    }
  }

  async function handleCreateSupplier(e: Event) {
    e.preventDefault();
    try {
      await fetchAdmin('/api/v1/admin/suppliers', {
        method: 'POST',
        body: JSON.stringify({
          name: newSupName,
          contact_person: newSupContact || null,
          phone: newSupPhone || null,
          email: newSupEmail || null,
          address: newSupAddress || null
        })
      });
      toast.success('Supplier baru berhasil didaftarkan');
      isAddSupplierOpen = false;
      newSupName = '';
      newSupContact = '';
      newSupPhone = '';
      newSupEmail = '';
      newSupAddress = '';
      await loadSuppliersAndPo();
    } catch (err: any) {
      toast.error(err.message || 'Gagal mendaftarkan supplier');
    }
  }

  async function handleCreatePo(e: Event) {
    e.preventDefault();
    if (!newPoSupplierId || !newPoProductId) {
      toast.error('Pilih supplier dan produk terlebih dahulu');
      return;
    }
    try {
      await fetchAdmin('/api/v1/admin/purchase-orders', {
        method: 'POST',
        body: JSON.stringify({
          supplier_id: newPoSupplierId,
          notes: newPoNotes || null,
          items: [{
            product_id: newPoProductId,
            quantity: newPoQuantity,
            unit_cost_cents: Math.round(newPoUnitCost * 100)
          }]
        })
      });
      toast.success('Purchase Order (PO) berhasil dibuat!');
      isAddPoOpen = false;
      newPoNotes = '';
      await loadSuppliersAndPo();
    } catch (err: any) {
      toast.error(err.message || 'Gagal membuat Purchase Order');
    }
  }

  async function handleReceivePo(poId: string) {
    if (!confirm('Konfirmasi penerimaan barang untuk PO ini? Stok produk akan otomatis bertambah secara real-time.')) return;
    try {
      await fetchAdmin(`/api/v1/admin/purchase-orders/${poId}/receive`, {
        method: 'POST'
      });
      toast.success('📦 Barang diterima & stok gudang otomatis diperbarui!');
      await loadSuppliersAndPo();
    } catch (err: any) {
      toast.error(err.message || 'Gagal memproses penerimaan PO');
    }
  }

  async function handleCancelPo(poId: string) {
    if (!confirm('Batalkan Purchase Order ini?')) return;
    try {
      await fetchAdmin(`/api/v1/admin/purchase-orders/${poId}/cancel`, {
        method: 'POST'
      });
      toast.success('Purchase Order berhasil dibatalkan.');
      await loadSuppliersAndPo();
    } catch (err: any) {
      toast.error(err.message || 'Gagal membatalkan PO');
    }
  }

  onMount(() => {
    loadSuppliersAndPo();
  });
</script>

<div class="suppliers-panel">
  <div class="panel-header-row">
    <div>
      <h2>🏢 Manajemen Supplier & Purchase Order (PO)</h2>
      <p class="panel-desc">Kelola data mitra pemasok dan otomatisasi penerimaan restock inventori gudang.</p>
    </div>
    <div class="action-btn-row">
      <button class="btn-action" onclick={() => isAddSupplierOpen = true}>➕ Tambah Mitra Supplier</button>
      <button class="btn-action primary" onclick={() => isAddPoOpen = true}>📦 Buat PO Restock</button>
    </div>
  </div>

  <!-- List of Purchase Orders -->
  <div class="table-section">
    <h3 class="section-title">
      📑 Riwayat Purchase Order (PO) ({poList.length})
    </h3>
    {#if supplierLoading}
      <p class="empty-text">Memuat riwayat PO...</p>
    {:else if poList.length === 0}
      <div class="empty-dashed-box">
        <p>Belum ada Purchase Order yang tercatat.</p>
      </div>
    {:else}
      <table class="admin-table">
        <thead>
          <tr>
            <th>No. PO</th>
            <th>Pemasok (Supplier)</th>
            <th>Jumlah Item</th>
            <th>Total Biaya (HPP)</th>
            <th>Status PO</th>
            <th>Tanggal Pesan</th>
            <th class="text-right">Aksi</th>
          </tr>
        </thead>
        <tbody>
          {#each poList as po (po.id)}
            <tr>
              <td><strong class="po-num">{po.po_number}</strong></td>
              <td>{po.supplier_name || 'Mitra Pemasok'}</td>
              <td>{po.items.reduce((acc, i) => acc + i.quantity, 0)} unit ({po.items.length} varian)</td>
              <td><strong>{formatRupiah(po.total_cost_cents / 100)}</strong></td>
              <td>
                {#if po.status === 'received'}
                  <span class="status-badge received">✓ Diterima & Masuk Stok</span>
                {:else if po.status === 'ordered'}
                  <span class="status-badge ordered">⏳ Menunggu Pengiriman</span>
                {:else}
                  <span class="status-badge cancelled">✕ Dibatalkan</span>
                {/if}
              </td>
              <td><small>{new Date(po.ordered_at).toLocaleDateString('id-ID')}</small></td>
              <td class="text-right">
                {#if po.status === 'ordered'}
                  <button class="btn-action primary btn-compact" onclick={() => handleReceivePo(po.id)}>
                    📥 Terima Barang
                  </button>
                  <button class="btn-action danger btn-compact" onclick={() => handleCancelPo(po.id)}>
                    ✕ Batal
                  </button>
                {:else if po.status === 'received'}
                  <small class="text-emerald">Selesai ({new Date(po.received_at || po.ordered_at).toLocaleDateString('id-ID')})</small>
                {:else}
                  <small class="text-muted">Tidak aktif</small>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>

  <!-- List of Suppliers -->
  <div>
    <h3 class="section-title">
      🏭 Direktori Mitra Supplier ({suppliersList.length})
    </h3>
    {#if suppliersList.length === 0}
      <div class="empty-dashed-box">
        <p>Belum ada supplier yang didaftarkan.</p>
      </div>
    {:else}
      <table class="admin-table">
        <thead>
          <tr>
            <th>Nama Perusahaan / Mitra</th>
            <th>Kontak Person (PIC)</th>
            <th>Telepon / WA</th>
            <th>Email</th>
            <th>Alamat Gudang / Pabrik</th>
            <th>Status</th>
          </tr>
        </thead>
        <tbody>
          {#each suppliersList as s (s.id)}
            <tr>
              <td><strong>{s.name}</strong></td>
              <td>{s.contact_person || '-'}</td>
              <td>{s.phone || '-'}</td>
              <td>{s.email || '-'}</td>
              <td><small>{s.address || '-'}</small></td>
              <td>
                {#if s.is_active}
                  <span class="dot-active">● Aktif</span>
                {:else}
                  <span class="dot-inactive">● Nonaktif</span>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>

<!-- Add Supplier Modal -->
{#if isAddSupplierOpen}
  <div class="modal-overlay" onclick={() => isAddSupplierOpen = false} role="button" tabindex="0" onkeydown={(e) => e.key === 'Escape' && (isAddSupplierOpen = false)}>
    <div class="modal-card" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" style="max-width: 500px;">
      <div class="modal-header">
        <h3>🏢 Tambah Mitra Supplier Baru</h3>
        <button class="close-btn" onclick={() => isAddSupplierOpen = false}>&times;</button>
      </div>
      <form onsubmit={handleCreateSupplier} class="modal-form">
        <label>
          Nama Perusahaan Supplier / Distributor:
          <input type="text" bind:value={newSupName} required placeholder="PT Sumber Logistik Prima" />
        </label>
        <div class="row-fields">
          <label>
            Contact Person (PIC):
            <input type="text" bind:value={newSupContact} placeholder="Budi Santoso" />
          </label>
          <label>
            Nomor Telepon / WhatsApp:
            <input type="text" bind:value={newSupPhone} placeholder="+62812345678" />
          </label>
        </div>
        <label>
          Email Perusahaan:
          <input type="email" bind:value={newSupEmail} placeholder="order@supplier.co.id" />
        </label>
        <label>
          Alamat Gudang / Kantor:
          <textarea bind:value={newSupAddress} rows="2" placeholder="Kawasan Industri MM2100, Cikarang Barat"></textarea>
        </label>
        <div class="modal-actions">
          <button type="button" class="btn-cancel" onclick={() => isAddSupplierOpen = false}>Batal</button>
          <button type="submit" class="btn-save">Daftarkan Supplier</button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Add PO Modal -->
{#if isAddPoOpen}
  <div class="modal-overlay" onclick={() => isAddPoOpen = false} role="button" tabindex="0" onkeydown={(e) => e.key === 'Escape' && (isAddPoOpen = false)}>
    <div class="modal-card" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" style="max-width: 520px;">
      <div class="modal-header">
        <h3>📦 Buat Purchase Order (PO) Restock</h3>
        <button class="close-btn" onclick={() => isAddPoOpen = false}>&times;</button>
      </div>
      <form onsubmit={handleCreatePo} class="modal-form">
        <label>
          Pilih Mitra Supplier:
          <select class="select-custom" bind:value={newPoSupplierId} required>
            <option value="">-- Pilih Supplier --</option>
            {#each suppliersList as s (s.id)}
              <option value={s.id}>{s.name} {s.contact_person ? `(${s.contact_person})` : ''}</option>
            {/each}
          </select>
        </label>
        <label>
          Pilih Produk yang Direstock:
          <select class="select-custom" bind:value={newPoProductId} required>
            <option value="">-- Pilih Produk Katalog --</option>
            {#each products as p (p.id)}
              <option value={p.id}>{p.name} (Stok Saat Ini: {p.stock})</option>
            {/each}
          </select>
        </label>
        <div class="row-fields">
          <label>
            Jumlah Unit Restock:
            <input type="number" bind:value={newPoQuantity} min="1" required />
          </label>
          <label>
            Biaya Beli per Unit (Rp):
            <input type="number" bind:value={newPoUnitCost} min="1000" step="1000" required />
          </label>
        </div>
        <label>
          Catatan PO (Opsional):
          <textarea bind:value={newPoNotes} rows="2" placeholder="Pengiriman ekspedisi estimasi 2 hari"></textarea>
        </label>
        <div class="total-po-box">
          Total Estimasi Biaya PO: <strong class="text-cyan">{formatRupiah(newPoQuantity * newPoUnitCost)}</strong>
        </div>
        <div class="modal-actions">
          <button type="button" class="btn-cancel" onclick={() => isAddPoOpen = false}>Batal</button>
          <button type="submit" class="btn-save">Terbitkan PO Restock</button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .suppliers-panel { width: 100%; }
  .panel-header-row {
    display: flex; justify-content: space-between; align-items: center;
    margin-bottom: 1.5rem; flex-wrap: wrap; gap: 1rem;
  }
  h2 { margin: 0; font-size: 1.25rem; color: #f8fafc; }
  .panel-desc { color: #94a3b8; font-size: 0.9rem; margin-top: 0.25rem; }
  .action-btn-row { display: flex; gap: 0.6rem; }
  .table-section { margin-bottom: 2.5rem; }
  .section-title {
    font-size: 1.15rem; margin-bottom: 1rem; color: #f8fafc; display: flex; align-items: center; gap: 0.5rem;
  }
  .empty-dashed-box {
    padding: 2rem; text-align: center; background: #1e293b; border-radius: 8px; border: 1px dashed #334155;
    color: #94a3b8;
  }
  .admin-table { width: 100%; border-collapse: collapse; }
  .admin-table th, .admin-table td {
    padding: 0.75rem 1rem; text-align: left; border-bottom: 1px solid #1e293b; font-size: 0.85rem;
  }
  .admin-table th { background: #1e293b; color: #94a3b8; font-size: 0.8rem; }
  .po-num { color: #38bdf8; font-family: monospace; }
  .status-badge { padding: 0.25rem 0.5rem; border-radius: 4px; font-size: 0.75rem; font-weight: 600; color: #fff; }
  .status-badge.received { background: #059669; }
  .status-badge.ordered { background: #d97706; }
  .status-badge.cancelled { background: #dc2626; }
  .text-right { text-align: right; }
  .text-emerald { color: #10b981; }
  .text-muted { color: #94a3b8; }
  .text-cyan { color: #38bdf8; }
  .dot-active { color: #10b981; font-weight: 600; font-size: 0.8rem; }
  .dot-inactive { color: #ef4444; font-weight: 600; font-size: 0.8rem; }
  .btn-compact { font-size: 0.8rem; padding: 0.35rem 0.65rem; margin-left: 0.3rem; }
  .btn-action {
    background: #334155; color: #fff; border: none; padding: 0.5rem 0.9rem;
    border-radius: 6px; cursor: pointer; font-size: 0.85rem; font-weight: 600;
  }
  .btn-action.primary { background: #0284c7; }
  .btn-action.danger { background: #dc2626; }
  .empty-text { color: #94a3b8; text-align: center; padding: 2rem 0; }

  /* Modal */
  .modal-overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.7);
    display: flex; align-items: center; justify-content: center; z-index: 1000;
  }
  .modal-card {
    background: #0f172a; border: 1px solid #334155; border-radius: 12px;
    width: 90%; padding: 1.5rem; color: #f8fafc;
  }
  .modal-header { display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid #334155; padding-bottom: 0.5rem; }
  .modal-header h3 { margin: 0; font-size: 1.15rem; color: #f8fafc; }
  .close-btn { background: none; border: none; font-size: 1.4rem; color: #94a3b8; cursor: pointer; }
  .modal-form { display: flex; flex-direction: column; gap: 0.9rem; margin-top: 1rem; }
  .modal-form label { font-size: 0.85rem; color: #cbd5e1; display: flex; flex-direction: column; gap: 0.25rem; }
  .modal-form input, .modal-form textarea {
    background: #1e293b; border: 1px solid #334155; border-radius: 6px;
    padding: 0.6rem; color: #fff; font-family: inherit;
  }
  .row-fields { display: flex; gap: 1rem; }
  .row-fields label { flex: 1; }
  .select-custom {
    background: #1e293b; border: 1px solid #334155; border-radius: 6px;
    padding: 0.6rem; color: #fff; font-family: inherit;
  }
  .total-po-box { background: #1e293b; padding: 0.75rem; border-radius: 6px; font-size: 0.85rem; color: #94a3b8; }
  .modal-actions { display: flex; justify-content: flex-end; gap: 0.75rem; margin-top: 0.75rem; }
  .btn-cancel { background: #334155; color: #fff; border: none; padding: 0.6rem 1.2rem; border-radius: 6px; cursor: pointer; }
  .btn-save { background: #059669; color: #fff; border: none; padding: 0.6rem 1.2rem; border-radius: 6px; font-weight: bold; cursor: pointer; }
</style>
