<script lang="ts">
  import { onMount } from 'svelte';
  import { toast } from '../toast.svelte';
  import { adminAuth } from '../adminAuth.svelte';
  import { fetchAdmin } from './adminApi';

  interface BackupFile {
    filename: string;
    size_bytes: number;
    created_at: string;
  }

  interface DatabaseHealth {
    status: string;
    engine: string;
    integrity: string;
  }

  let backups = $state<BackupFile[]>([]);
  let dbHealth = $state<DatabaseHealth | null>(null);
  let backupLoading = $state(false);
  let creatingBackup = $state(false);

  export async function loadBackupsAndHealth() {
    backupLoading = true;
    try {
      const [bList, health] = await Promise.all([
        fetchAdmin<BackupFile[]>('/api/v1/admin/database/backups').catch(() => []),
        fetchAdmin<DatabaseHealth>('/api/v1/admin/database/health').catch(() => null)
      ]);
      backups = bList || [];
      dbHealth = health || null;
    } catch (err: any) {
      toast.error(err.message || 'Gagal memuat status database & backup');
    } finally {
      backupLoading = false;
    }
  }

  async function handleCreateBackup() {
    creatingBackup = true;
    try {
      const res = await fetchAdmin<BackupFile>('/api/v1/admin/database/backup', { method: 'POST' });
      toast.success(`Backup berhasil dibuat: ${res.filename}`);
      await loadBackupsAndHealth();
    } catch (err: any) {
      toast.error(err.message || 'Gagal membuat backup');
    } finally {
      creatingBackup = false;
    }
  }

  function downloadBackup(filename: string) {
    if (!adminAuth.token) return;
    const url = `/api/v1/admin/database/backups/${encodeURIComponent(filename)}/download`;
    fetch(url, {
      headers: {
        'Authorization': `Bearer ${adminAuth.token}`
      }
    })
    .then(async (res) => {
      if (!res.ok) throw new Error('Gagal mendownload backup');
      const blob = await res.blob();
      const blobUrl = window.URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = blobUrl;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      a.remove();
      window.URL.revokeObjectURL(blobUrl);
    })
    .catch((err) => {
      toast.error(err.message || 'Gagal mendownload backup');
    });
  }

  onMount(() => {
    loadBackupsAndHealth();
  });
</script>

<div class="card-panel">
  <div class="panel-header">
    <h2>💾 Database & Backup Manager</h2>
    <div class="header-actions">
      <button
        class="btn-refresh"
        onclick={loadBackupsAndHealth}
        disabled={backupLoading}
      >
        {backupLoading ? 'Memeriksa...' : '🔄 Refresh Status'}
      </button>
      <button
        class="btn-add-prod"
        onclick={handleCreateBackup}
        disabled={creatingBackup}
      >
        {creatingBackup ? 'Membuat Backup...' : '⚡ Buat Backup Sekarang'}
      </button>
    </div>
  </div>

  <!-- Health Status Banner -->
  {#if dbHealth}
    <div class="db-health-banner">
      <div class="db-health-icon">
        {dbHealth.status === 'healthy' ? '🟢' : '🟡'}
      </div>
      <div class="db-health-info">
        <div class="db-health-title">
          Status Database: <span class="db-status-tag" class:healthy={dbHealth.status === 'healthy'}>{dbHealth.status}</span>
        </div>
        <div class="db-health-meta">
          <span>Engine: <strong class="text-white">{dbHealth.engine}</strong></span>
          <span>Integritas: <strong class="text-white">PRAGMA integrity_check = {dbHealth.integrity}</strong></span>
        </div>
      </div>
    </div>
  {/if}

  <h3 class="section-subheading">📁 Riwayat Berkas Backup Snapshot ({backups.length})</h3>

  {#if backupLoading}
    <p class="empty-text">Memuat daftar backup...</p>
  {:else if backups.length === 0}
    <div class="empty-state">
      <div class="empty-icon">📭</div>
      <p>Belum ada berkas backup yang tersimpan di server.</p>
      <small>Klik tombol "⚡ Buat Backup Sekarang" di kanan atas untuk membuat snapshot SQLite instan.</small>
    </div>
  {:else}
    <table class="table-custom">
      <thead>
        <tr>
          <th>Nama Berkas</th>
          <th>Ukuran</th>
          <th>Waktu Dibuat</th>
          <th class="text-right">Aksi</th>
        </tr>
      </thead>
      <tbody>
        {#each backups as b (b.filename)}
          <tr>
            <td>
              <strong class="backup-filename">{b.filename}</strong>
            </td>
            <td>{(b.size_bytes / 1024).toFixed(1)} KB</td>
            <td>
              <small>{new Date(b.created_at).toLocaleString('id-ID')}</small>
            </td>
            <td class="text-right">
              <button
                class="btn-action primary"
                onclick={() => downloadBackup(b.filename)}
              >
                ⬇️ Unduh Berkas
              </button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .card-panel { width: 100%; }
  .panel-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
  h2 { margin: 0; font-size: 1.25rem; color: #f8fafc; }
  .header-actions { display: flex; gap: 0.5rem; }
  .btn-refresh {
    background: #0284c7; color: #fff; border: none; padding: 0.45rem 0.9rem;
    border-radius: 6px; cursor: pointer; font-size: 0.85rem; font-weight: 600;
  }
  .btn-add-prod {
    background: #059669; color: #fff; border: none; padding: 0.55rem 1rem;
    border-radius: 6px; font-weight: bold; cursor: pointer;
  }
  .db-health-banner {
    display: flex; gap: 1.5rem; align-items: center; background: #0f172a;
    border: 1px solid #1e293b; border-radius: 8px; padding: 1rem 1.25rem; margin-bottom: 1.5rem;
  }
  .db-health-icon { font-size: 2rem; }
  .db-health-info { flex: 1; }
  .db-health-title { font-weight: 700; font-size: 1.05rem; color: #f8fafc; margin-bottom: 0.25rem; }
  .db-status-tag { text-transform: uppercase; color: #facc15; }
  .db-status-tag.healthy { color: #4ade80; }
  .db-health-meta { font-size: 0.85rem; color: #94a3b8; display: flex; gap: 1.5rem; }
  .text-white { color: #cbd5e1; }
  .section-subheading { font-size: 1rem; color: #94a3b8; margin-bottom: 0.75rem; }
  .table-custom { width: 100%; border-collapse: collapse; margin-top: 1rem; }
  .table-custom th, .table-custom td {
    padding: 0.75rem 1rem; text-align: left; border-bottom: 1px solid #1e293b; font-size: 0.9rem;
  }
  .table-custom th { background: #1e293b; color: #94a3b8; font-size: 0.8rem; }
  .text-right { text-align: right; }
  .backup-filename { color: #38bdf8; font-family: monospace; }
  .btn-action {
    background: #334155; color: #fff; border: none; padding: 0.35rem 0.7rem;
    border-radius: 4px; cursor: pointer; font-size: 0.8rem;
  }
  .btn-action.primary { background: #0284c7; font-weight: bold; }
  .empty-state { text-align: center; padding: 3rem 1rem; color: #64748b; }
  .empty-icon { font-size: 3rem; margin-bottom: 0.5rem; }
  .empty-text { color: #94a3b8; text-align: center; padding: 2rem 0; }
</style>
