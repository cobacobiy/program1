<script lang="ts">
  import type { HealthResponse } from '../types';

  interface Props {
    healthData: HealthResponse | null;
    healthLoading: boolean;
    healthError: string | null;
    onRefresh: () => void;
  }

  let { healthData, healthLoading, healthError, onRefresh }: Props = $props();
</script>

<section class="diagnostic-view">
  <div class="card-panel">
    <div class="panel-header">
      <h3>⚡ Status Backend Axum (<code>GET /health</code>)</h3>
      <button class="btn-refresh" onclick={onRefresh} disabled={healthLoading}>
        {healthLoading ? 'Memeriksa...' : 'Ping Server'}
      </button>
    </div>

    {#if healthLoading}
      <p class="info-text">Menghubungi endpoint backend...</p>
    {:else if healthError}
      <div class="alert-box error">
        <h4>❌ Sambungan Terputus</h4>
        <p>{healthError}</p>
      </div>
    {:else if healthData}
      <div class="alert-box success">
        <h4>✅ Server Rust Axum Berjalan Normal!</h4>
        <p><strong>Status API:</strong> <code>{healthData.status}</code></p>
        <p><strong>Versi Binary:</strong> <code>{healthData.version}</code></p>
        {#if healthData.subsystems}
          <div class="subsystems-dump">
            <strong>Subsystem Diagnostic:</strong>
            <pre>{JSON.stringify(healthData.subsystems, null, 2)}</pre>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</section>

<style>
  .diagnostic-view { width: 100%; }
  .card-panel {
    background: #1e293b; border: 1px solid #334155; border-radius: 12px; padding: 1.5rem;
  }
  .panel-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
  .panel-header h3 { margin: 0; color: #f8fafc; font-size: 1.15rem; }
  .panel-header code { background: #0f172a; padding: 0.2rem 0.4rem; border-radius: 4px; color: #38bdf8; }
  .btn-refresh {
    background: #0284c7; color: #fff; border: none; padding: 0.5rem 1rem;
    border-radius: 6px; cursor: pointer; font-weight: 500; transition: background 0.15s;
  }
  .btn-refresh:hover:not(:disabled) { background: #0369a1; }
  .btn-refresh:disabled { opacity: 0.6; cursor: not-allowed; }
  .info-text { color: #94a3b8; }
  .alert-box { padding: 1.25rem; border-radius: 8px; margin-top: 1rem; }
  .alert-box h4 { margin: 0 0 0.5rem; font-size: 1rem; }
  .alert-box.success { background: #064e3b; color: #a7f3d0; }
  .alert-box.error { background: #7f1d1d; color: #fecaca; }
  .subsystems-dump { margin-top: 0.75rem; }
  .subsystems-dump pre { background: #022c22; padding: 0.75rem; border-radius: 6px; overflow-x: auto; color: #a7f3d0; }
</style>
