<script lang="ts">
  interface AuditLogRecord {
    id: string;
    actor_username?: string;
    action: string;
    resource_type: string;
    created_at: string;
  }

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

<style>
  .section-panel { width: 100%; }
  h2 { margin: 0 0 1rem; font-size: 1.25rem; color: #f8fafc; }
  .table-custom { width: 100%; border-collapse: collapse; margin-top: 1rem; }
  .table-custom th, .table-custom td {
    padding: 0.75rem 1rem; text-align: left; border-bottom: 1px solid #1e293b; font-size: 0.9rem;
  }
  .table-custom th { background: #1e293b; color: #94a3b8; font-size: 0.8rem; }
  .table-custom code { background: #1e293b; padding: 0.2rem 0.4rem; border-radius: 4px; color: #38bdf8; }
  .empty-text { color: #94a3b8; text-align: center; padding: 2rem 0; }
</style>
