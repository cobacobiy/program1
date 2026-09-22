<script lang="ts">
  import { onMount } from 'svelte';
  import type { ReturnRequest, ProcessReturnPayload, UpdateReturnStatusPayload } from '../types';
  import { formatRupiah } from '../currency';
  import { toast } from '../toast.svelte';
  import { fetchAdmin } from './adminApi';

  let adminReturns = $state<ReturnRequest[]>([]);
  let adminReturnsLoading = $state(false);
  let returnStatusFilter = $state<string>('');
  let rejectModalOpen = $state(false);
  let rejectTargetId = $state('');
  let rejectNotes = $state('');
  let isProcessingReturn = $state(false);

  export async function loadAdminReturns() {
    adminReturnsLoading = true;
    try {
      const url = returnStatusFilter
        ? `/api/v1/admin/returns?status=${encodeURIComponent(returnStatusFilter)}`
        : '/api/v1/admin/returns';
      const res = await fetchAdmin<ReturnRequest[]>(url);
      adminReturns = res || [];
    } catch (err: any) {
      toast.error(err.message || 'Gagal memuat daftar retur.');
    } finally {
      adminReturnsLoading = false;
    }
  }

  function openRejectModal(id: string) {
    rejectTargetId = id;
    rejectNotes = '';
    rejectModalOpen = true;
  }

  async function processReturn(id: string, action: 'approve' | 'reject', notes?: string, override?: number) {
    isProcessingReturn = true;
    try {
      const payload: ProcessReturnPayload = {
        action,
        admin_notes: notes,
        refund_amount_override: override,
      };
      await fetchAdmin<ReturnRequest>(`/api/v1/admin/returns/${id}/process`, {
        method: 'POST',
        body: JSON.stringify(payload),
      });
      toast.success(action === 'approve' ? 'Pengajuan retur disetujui!' : 'Pengajuan retur ditolak.');
      rejectModalOpen = false;
      await loadAdminReturns();
    } catch (err: any) {
      toast.error(err.message || 'Gagal memproses retur.');
    } finally {
      isProcessingReturn = false;
    }
  }

  async function updateReturnStatus(id: string, status: 'return_shipped' | 'received' | 'refunded') {
    try {
      const payload: UpdateReturnStatusPayload = { status };
      await fetchAdmin<ReturnRequest>(`/api/v1/admin/returns/${id}/status`, {
        method: 'POST',
        body: JSON.stringify(payload),
      });
      toast.success(`Status retur diperbarui menjadi ${status.toUpperCase()}!`);
      await loadAdminReturns();
    } catch (err: any) {
      toast.error(err.message || 'Gagal memperbarui status retur.');
    }
  }

  onMount(() => {
    loadAdminReturns();
  });
</script>

<div class="card-panel">
  <div class="panel-header">
    <h2>🔄 Manajemen Retur & Pengembalian Dana (Refund)</h2>
    <div class="header-actions">
      <button class="btn-refresh" onclick={loadAdminReturns} disabled={adminReturnsLoading}>
        {adminReturnsLoading ? 'Memuat...' : '🔄 Refresh'}
      </button>
    </div>
  </div>

  <!-- Status Filter Pills -->
  <div class="filter-pills-row">
    <button class="pill-btn" class:active={returnStatusFilter === ''} onclick={() => { returnStatusFilter = ''; loadAdminReturns(); }}>
      Semua ({adminReturns.length})
    </button>
    <button class="pill-btn" class:active={returnStatusFilter === 'pending'} onclick={() => { returnStatusFilter = 'pending'; loadAdminReturns(); }}>
      Menunggu Review
    </button>
    <button class="pill-btn" class:active={returnStatusFilter === 'approved'} onclick={() => { returnStatusFilter = 'approved'; loadAdminReturns(); }}>
      Disetujui
    </button>
    <button class="pill-btn" class:active={returnStatusFilter === 'return_shipped'} onclick={() => { returnStatusFilter = 'return_shipped'; loadAdminReturns(); }}>
      Dalam Pengiriman
    </button>
    <button class="pill-btn" class:active={returnStatusFilter === 'received'} onclick={() => { returnStatusFilter = 'received'; loadAdminReturns(); }}>
      Diterima & Restock
    </button>
    <button class="pill-btn" class:active={returnStatusFilter === 'refunded'} onclick={() => { returnStatusFilter = 'refunded'; loadAdminReturns(); }}>
      Selesai (Refunded)
    </button>
    <button class="pill-btn" class:active={returnStatusFilter === 'rejected'} onclick={() => { returnStatusFilter = 'rejected'; loadAdminReturns(); }}>
      Ditolak
    </button>
  </div>

  {#if adminReturnsLoading}
    <p class="empty-text">Memuat daftar permintaan retur...</p>
  {:else if adminReturns.length === 0}
    <p class="empty-text">Belum ada pengajuan retur untuk status yang dipilih.</p>
  {:else}
    <table class="table-custom">
      <thead>
        <tr>
          <th>ID / Tanggal</th>
          <th>ID Order</th>
          <th>ID Pembeli</th>
          <th>Alasan Retur</th>
          <th>Estimasi Refund</th>
          <th>Status</th>
          <th>Bukti</th>
          <th>Aksi Admin</th>
        </tr>
      </thead>
      <tbody>
        {#each adminReturns as r (r.id)}
          <tr>
            <td>
              <strong style="color: #38bdf8;">#{r.id.slice(0, 8)}</strong>
              <br />
              <small>{new Date(r.created_at).toLocaleDateString('id-ID')}</small>
            </td>
            <td>
              <code>#{r.order_id.slice(0, 8)}</code>
            </td>
            <td>
              <small>{r.buyer_id.slice(0, 12)}</small>
            </td>
            <td>
              <strong>{r.reason}</strong>
              {#if r.description}
                <p style="font-size: 0.8rem; color: #94a3b8; margin: 0.2rem 0 0 0;">{r.description}</p>
              {/if}
            </td>
            <td>
              <strong style="color: #34d399;">{formatRupiah(r.refund_amount_cents)}</strong>
            </td>
            <td>
              <span class="status-pill {r.status.toLowerCase()}">
                {r.status.toUpperCase()}
              </span>
            </td>
            <td>
              {#if r.evidence_urls && r.evidence_urls.length > 0}
                <a href={r.evidence_urls[0]} target="_blank" rel="noopener noreferrer" style="color: #38bdf8; font-size: 0.82rem; text-decoration: none;">
                  🖼️ Lihat Foto
                </a>
              {:else}
                <span style="color: #64748b;">-</span>
              {/if}
            </td>
            <td>
              <div style="display: flex; gap: 0.35rem; flex-wrap: wrap;">
                {#if r.status === 'pending'}
                  <button
                    class="btn-action primary"
                    onclick={() => processReturn(r.id, 'approve')}
                    disabled={isProcessingReturn}
                  >
                    ✓ Setujui
                  </button>
                  <button
                    class="btn-action danger"
                    onclick={() => openRejectModal(r.id)}
                    disabled={isProcessingReturn}
                  >
                    ✕ Tolak
                  </button>
                {:else if r.status === 'approved'}
                  <button
                    class="btn-action primary"
                    onclick={() => updateReturnStatus(r.id, 'return_shipped')}
                  >
                    🚚 Pengiriman
                  </button>
                {:else if r.status === 'return_shipped'}
                  <button
                    class="btn-action primary"
                    onclick={() => updateReturnStatus(r.id, 'received')}
                  >
                    📦 Terima & Restock
                  </button>
                {:else if r.status === 'received'}
                  <button
                    class="btn-action success"
                    onclick={() => updateReturnStatus(r.id, 'refunded')}
                  >
                    💸 Refund Selesai
                  </button>
                {:else}
                  <small style="color: #64748b;">Proses Selesai</small>
                {/if}
              </div>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<!-- Modal Tolak Retur -->
{#if rejectModalOpen}
  <div class="modal-overlay" onclick={() => rejectModalOpen = false} role="button" tabindex="0" onkeydown={(e) => e.key === 'Escape' && (rejectModalOpen = false)}>
    <div class="modal-card" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" style="max-width: 440px;">
      <div class="modal-header">
        <h3>✕ Alasan Penolakan Retur</h3>
        <button class="close-btn" onclick={() => rejectModalOpen = false}>&times;</button>
      </div>
      <div style="display: flex; flex-direction: column; gap: 0.85rem; margin-top: 1rem;">
        <label style="font-size: 0.85rem; color: #cbd5e1; font-weight: 600;">
          Berikan catatan/alasan ke pembeli (wajib):
          <textarea
            bind:value={rejectNotes}
            rows="3"
            placeholder="Contoh: Garansi resmi telah lewat / Kerusakan akibat kesalahan pengguna."
            style="width: 100%; margin-top: 0.35rem; background: #1e293b; border: 1px solid #334155; border-radius: 6px; padding: 0.6rem; color: #fff;"
          ></textarea>
        </label>
        <div style="display: flex; justify-content: flex-end; gap: 0.5rem; margin-top: 0.5rem;">
          <button type="button" class="btn-action" onclick={() => rejectModalOpen = false}>Batal</button>
          <button
            type="button"
            class="btn-action danger"
            onclick={() => processReturn(rejectTargetId, 'reject', rejectNotes)}
            disabled={isProcessingReturn || !rejectNotes.trim()}
          >
            {isProcessingReturn ? 'Memproses...' : 'Tolak Retur'}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .card-panel { width: 100%; }
  .panel-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
  h2 { margin: 0; font-size: 1.25rem; color: #f8fafc; }
  .btn-refresh {
    background: #0284c7; color: #fff; border: none; padding: 0.45rem 0.9rem;
    border-radius: 6px; cursor: pointer; font-size: 0.85rem; font-weight: 600;
  }
  .filter-pills-row { margin-bottom: 1rem; display: flex; gap: 0.5rem; flex-wrap: wrap; }
  .pill-btn {
    background: #1e293b; color: #94a3b8; border: 1px solid #334155;
    padding: 0.35rem 0.75rem; border-radius: 20px; font-size: 0.8rem; cursor: pointer;
  }
  .pill-btn.active { background: #0284c7; color: #fff; border-color: #0284c7; font-weight: 600; }
  .table-custom { width: 100%; border-collapse: collapse; margin-top: 1rem; }
  .table-custom th, .table-custom td {
    padding: 0.75rem 1rem; text-align: left; border-bottom: 1px solid #1e293b; font-size: 0.9rem;
  }
  .table-custom th { background: #1e293b; color: #94a3b8; font-size: 0.8rem; }
  .status-pill { padding: 0.2rem 0.5rem; border-radius: 4px; font-size: 0.75rem; font-weight: bold; }
  .status-pill.pending { background: #854d0e; color: #fef08a; }
  .status-pill.approved { background: #0284c7; color: #e0f2fe; }
  .status-pill.return_shipped { background: #7c3aed; color: #ede9fe; }
  .status-pill.received { background: #059669; color: #ecfdf5; }
  .status-pill.refunded { background: #047857; color: #a7f3d0; }
  .status-pill.rejected { background: #7f1d1d; color: #fecaca; }
  .btn-action {
    background: #334155; color: #fff; border: none; padding: 0.35rem 0.7rem;
    border-radius: 4px; cursor: pointer; font-size: 0.8rem;
  }
  .btn-action.primary { background: #0284c7; font-weight: bold; }
  .btn-action.danger { background: #dc2626; font-weight: bold; }
  .btn-action.success { background: #10b981; font-weight: bold; }
  .btn-action:disabled { opacity: 0.5; cursor: not-allowed; }
  .empty-text { color: #94a3b8; text-align: center; padding: 2rem 0; }

  /* Modal */
  .modal-overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.7);
    display: flex; align-items: center; justify-content: center; z-index: 1000;
  }
  .modal-card {
    background: #0f172a; border: 1px solid #334155; border-radius: 12px;
    padding: 1.5rem; color: #f8fafc;
  }
  .modal-header { display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid #334155; padding-bottom: 0.5rem; }
  .modal-header h3 { margin: 0; font-size: 1.15rem; color: #f8fafc; }
  .close-btn { background: none; border: none; font-size: 1.4rem; color: #94a3b8; cursor: pointer; }
</style>
