<script lang="ts">
  import { auth } from './auth.svelte';

  let activeTab = $state<'login' | 'register'>('login');
  let email = $state('');
  let password = $state('');
  let name = $state('');
  let phone = $state('');

  async function handleSubmit(e: Event) {
    e.preventDefault();
    if (activeTab === 'login') {
      await auth.login(email, password);
    } else {
      const ok = await auth.register(name, email, phone, password);
      if (ok) activeTab = 'login';
    }
  }
</script>

{#if auth.isModalOpen}
  <div class="modal-overlay" onclick={() => auth.isModalOpen = false} role="button" tabindex="0" onkeydown={(e) => e.key === 'Escape' && (auth.isModalOpen = false)}>
    <div class="modal-card" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <div class="tab-header">
        <button class:active={activeTab === 'login'} onclick={() => activeTab = 'login'}>Masuk</button>
        <button class:active={activeTab === 'register'} onclick={() => activeTab = 'register'}>Daftar Akun</button>
        <button class="close-btn" onclick={() => auth.isModalOpen = false} aria-label="Tutup">&times;</button>
      </div>

      <form onsubmit={handleSubmit} class="form-body">
        {#if activeTab === 'register'}
          <label>
            Nama Lengkap:
            <input type="text" bind:value={name} required placeholder="Contoh: Budi Santoso" />
          </label>
          <label>
            Nomor Telepon / WhatsApp:
            <input type="tel" bind:value={phone} required placeholder="081234567890" />
          </label>
        {/if}

        <label>
          Email / Nomor HP:
          <input type="text" bind:value={email} required placeholder="nama@email.com" />
        </label>

        <label>
          Password:
          <input type="password" bind:value={password} required minlength="6" placeholder="••••••" />
        </label>

        <button type="submit" class="submit-btn" disabled={auth.isLoading}>
          {auth.isLoading ? 'Memproses...' : activeTab === 'login' ? 'Masuk Sekarang' : 'Daftar Sekarang'}
        </button>
      </form>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.7);
    display: flex; align-items: center; justify-content: center; z-index: 1000;
  }
  .modal-card {
    background: #0f172a; border: 1px solid #334155; border-radius: 12px;
    width: 90%; max-width: 420px; padding: 1.5rem; color: #f8fafc;
    box-shadow: 0 10px 30px rgba(0,0,0,0.5);
  }
  .tab-header {
    display: flex; gap: 0.5rem; border-bottom: 1px solid #334155; margin-bottom: 1.25rem;
    position: relative;
  }
  .tab-header button:not(.close-btn) {
    flex: 1; background: none; border: none; color: #94a3b8; padding: 0.6rem;
    font-size: 1rem; cursor: pointer; border-bottom: 2px solid transparent;
  }
  .tab-header button.active { color: #38bdf8; border-bottom-color: #38bdf8; font-weight: bold; }
  .close-btn {
    background: none; border: none; color: #94a3b8; font-size: 1.5rem; cursor: pointer;
    position: absolute; right: 0; top: -5px;
  }
  .form-body { display: flex; flex-direction: column; gap: 0.9rem; }
  label { font-size: 0.85rem; color: #cbd5e1; display: flex; flex-direction: column; gap: 0.25rem; }
  input {
    background: #1e293b; border: 1px solid #334155; border-radius: 6px;
    padding: 0.65rem; color: #fff; font-size: 0.95rem;
  }
  input:focus { border-color: #0284c7; outline: none; }
  .submit-btn {
    background: #0284c7; color: white; border: none; padding: 0.75rem;
    border-radius: 6px; font-weight: bold; font-size: 1rem; cursor: pointer; margin-top: 0.5rem;
  }
  .submit-btn:hover:not(:disabled) { background: #0369a1; }
  .submit-btn:disabled { background: #475569; cursor: not-allowed; }
</style>
