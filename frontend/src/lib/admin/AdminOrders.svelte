<script lang="ts">
  import { formatRupiah } from '../currency';
  import { toast } from '../toast.svelte';
  import { i18n } from '../i18n.svelte';
  import { fetchAdmin } from './adminApi';

  interface AdminOrder {
    id: string;
    total_amount_cents: number;
    status: string;
    created_at: string;
    tracking_number?: string | null;
  }

  interface Props {
    orders: AdminOrder[];
    loading: boolean;
    onRefresh: () => Promise<void> | void;
  }

  let { orders, loading, onRefresh }: Props = $props();

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
      await onRefresh();
    } catch (err: any) {
      toast.error(err.message || 'Gagal mengubah status pesanan');
    }
  }

  async function editTrackingNumber(orderId: string, currentResi: string | null | undefined) {
    const newResi = prompt('Masukkan Nomor Resi Pengiriman Baru:', currentResi || '');
    if (!newResi || newResi.trim() === '') return;
    try {
      await fetchAdmin(`/orders/${orderId}/tracking`, {
        method: 'PUT',
        body: JSON.stringify({ tracking_number: newResi.trim() }),
      });
      toast.success(`Nomor resi pesanan #${orderId.slice(0, 8)} diperbarui!`);
      await onRefresh();
    } catch (err: any) {
      toast.error(err.message || 'Gagal memperbarui nomor resi');
    }
  }
</script>

<div class="section-panel">
  <h2>🛒 {i18n.t('admin.orders_mgmt', 'Pesanan Masuk')}</h2>
  {#if loading}
    <p class="empty-text">Memuat pesanan...</p>
  {:else if orders.length === 0}
    <p class="empty-text">Belum ada pesanan masuk.</p>
  {:else}
    <table class="table-custom">
      <thead>
        <tr>
          <th>ID Pesanan</th>
          <th>Waktu</th>
          <th>Total</th>
          <th>Nomor Resi Pengiriman</th>
          <th>Status</th>
          <th>Aksi Perubahan Status</th>
        </tr>
      </thead>
      <tbody>
        {#each orders as o (o.id)}
          <tr>
            <td><code>#{o.id.slice(0, 8)}</code></td>
            <td>{new Date(o.created_at).toLocaleString()}</td>
            <td><strong>{formatRupiah(o.total_amount_cents)}</strong></td>
            <td>
              {#if o.tracking_number}
                <span class="tracking-code">{o.tracking_number}</span>
                <button class="btn-action edit-resi-btn" onclick={() => editTrackingNumber(o.id, o.tracking_number)}>✏️ Edit</button>
              {:else if o.status === 'SHIPPED' || o.status === 'DELIVERED'}
                <button class="btn-action edit-resi-btn" onclick={() => editTrackingNumber(o.id, null)}>+ Input Resi</button>
              {:else}
                <span class="empty-dash">-</span>
              {/if}
            </td>
            <td>
              <span class="status-pill {o.status.toLowerCase()}">{o.status}</span>
            </td>
            <td>
              {#if o.status === 'PENDING'}
                <button class="btn-action" onclick={() => updateOrderStatus(o.id, 'PAID')}>Tandai PAID</button>
              {:else if o.status === 'PAID'}
                <button class="btn-action" onclick={() => updateOrderStatus(o.id, 'PROCESSING')}>Proses (PROCESSING)</button>
              {:else if o.status === 'PROCESSING'}
                <button class="btn-action primary" onclick={() => updateOrderStatus(o.id, 'SHIPPED')}>Kirim & Input Resi</button>
              {:else if o.status === 'SHIPPED'}
                <button class="btn-action" onclick={() => updateOrderStatus(o.id, 'DELIVERED')}>Tandai Diterima (DELIVERED)</button>
              {:else}
                <span class="done-mark">✓ Selesai</span>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .section-panel { width: 100%; }
  h2 { margin: 0 0 1rem; font-size: 1.25rem; color: #f8fafc; }
  .table-custom { width: 100%; border-collapse: collapse; margin-top: 1rem; }
  .table-custom th, .table-custom td {
    padding: 0.75rem 1rem; text-align: left; border-bottom: 1px solid #1e293b; font-size: 0.9rem;
  }
  .table-custom th { background: #1e293b; color: #94a3b8; font-size: 0.8rem; }
  .tracking-code { font-family: monospace; color: #38bdf8; font-weight: bold; }
  .edit-resi-btn { margin-left: 0.5rem; font-size: 0.75rem; padding: 0.2rem 0.4rem; }
  .empty-dash { color: #64748b; font-size: 0.8rem; }
  .status-pill { padding: 0.2rem 0.5rem; border-radius: 4px; font-size: 0.75rem; font-weight: bold; }
  .status-pill.pending { background: #854d0e; color: #fef08a; }
  .status-pill.paid { background: #0284c7; color: #e0f2fe; }
  .status-pill.processing { background: #d97706; color: #fef3c7; }
  .status-pill.shipped { background: #7c3aed; color: #ede9fe; }
  .status-pill.delivered { background: #065f46; color: #a7f3d0; }
  .status-pill.cancelled { background: #7f1d1d; color: #fecaca; }
  .btn-action {
    background: #334155; color: #fff; border: none; padding: 0.35rem 0.7rem;
    border-radius: 4px; cursor: pointer; font-size: 0.8rem;
  }
  .btn-action.primary { background: #0284c7; font-weight: bold; }
  .done-mark { color: #10b981; font-weight: bold; }
  .empty-text { color: #94a3b8; text-align: center; padding: 2rem 0; }
</style>
