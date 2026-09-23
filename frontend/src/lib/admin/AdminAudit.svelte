<script lang="ts">
  import type { AuditLogRecord } from '../types';
  import './admin-shared.css';

  interface Props {
    auditLogs: AuditLogRecord[];
  }

  let { auditLogs }: Props = $props();
</script>

<div class="section-panel">
  <h2>📜 Riwayat Audit Trail Sistem</h2>
  {#if auditLogs.length === 0}
    <p class="empty-text">Belum ada log audit tercatat.</p>
  {:else}
    <div style="overflow-x: auto;">
      <table class="table-custom">
        <thead>
          <tr>
            <th>Waktu</th>
            <th>Aktor</th>
            <th>Aksi</th>
            <th>Tipe Resource</th>
          </tr>
        </thead>
        <tbody>
          {#each auditLogs as a (a.id)}
            <tr>
              <td><small>{new Date(a.created_at).toLocaleString('id-ID')}</small></td>
              <td><strong>{a.actor_username || 'Sistem'}</strong></td>
              <td><code>{a.action}</code></td>
              <td>{a.resource_type}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style>
  .table-custom code { background: #1e293b; padding: 0.2rem 0.4rem; border-radius: 4px; color: #38bdf8; }
</style>
