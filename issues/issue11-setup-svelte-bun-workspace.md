# Issue 11: Setup Frontend Workspace (Bun + Svelte 5 + Vite) & Proxy ke Rust Axum

> **Prioritas:** 🔴 CRITICAL (Fondasi)  
> **Estimasi:** 2-3 hari  
> **Kesulitan:** ⭐⭐ Dasar  
> **Prerequisite:** Tidak ada — fondasi awal untuk migrasi Svelte

---

## 📋 Deskripsi

Saat ini antarmuka web `program1` menggunakan file statis Vanilla HTML, CSS, dan JavaScript murni di folder `crates/web/static/`.  
Untuk meningkatkan pengalaman pengguna (reaktivitas tinggi, transisi mulus, manajemen state yang bersih) dan kemudahan pengembangan, kita akan memodernisasi frontend menggunakan kombinasi **Svelte 5** dan runtime **Bun** dengan build tool **Vite**.

### 💡 Konsep Arsitektur:
1. **Frontend (Svelte 5 + TypeScript + Bun):** Kode UI dibuat dengan komponen Svelte yang reaktif. Saat mode development, Vite dev server berjalan dengan Hot Module Replacement (HMR).
2. **Jembatan Komunikasi (Vite Proxy & JSON):** Semua pemanggilan API (`/api/*`, `/auth/*`, `/catalog/*`, `/v1/*`, `/uploads/*`, `/health`) otomatis di-proxy oleh Vite ke server backend Rust Axum di `http://localhost:8080` (atau port yang ditentukan). Tidak ada masalah CORS saat dev.
3. **Backend (Rust Axum):** Backend tetap menjadi sumber kebenaran (Source of Truth), mengeksekusi logika bisnis dan database SQLite via SQLx.

---

## 🎯 Acceptance Criteria

- [ ] Bun terpasang dan berfungsi sebagai package manager dan script runner.
- [ ] Folder `frontend/` terinisialisasi dengan template Vite + Svelte (TypeScript).
- [ ] Vite dev server berhasil dikonfigurasi dengan reverse proxy ke backend Rust Axum (`http://localhost:8080`).
- [ ] Command `bun run dev` dapat menjalankan server frontend di `http://localhost:5173`.
- [ ] Tersedia halaman demo sederhana yang memanggil endpoint `GET /health` ke backend Rust dan menampilkan status `"healthy"` secara reaktif.
- [ ] Command `bun run build` berhasil mengompilasi aset statis ke folder output yang ditentukan.
- [ ] Skrip `scripts/check_dependencies.sh` diperbarui untuk memverifikasi instalasi `bun`.

---

## 📐 Langkah-Langkah Pengerjaan

### Langkah 1: Pastikan Bun Terpasang di Sistem

Buka terminal dan jalankan perintah berikut untuk memeriksa apakah Bun sudah terpasang:
```bash
bun --version
```
Jika belum terpasang, pasang Bun dengan perintah resmi:
```bash
curl -fsSL https://bun.sh/install | bash
source ~/.bashrc  # atau source ~/.zshrc
bun --version
```

---

### Langkah 2: Inisialisasi Proyek Svelte di Folder `frontend/`

Dari root direktori proyek `program1`, inisialisasi proyek Svelte menggunakan template Vite resmi:
```bash
# Inisialisasi template svelte-ts ke folder frontend
bun create vite frontend --template svelte-ts

# Masuk ke folder frontend dan install dependensi
cd frontend
bun install
```

---

### Langkah 3: Konfigurasi `frontend/vite.config.ts` (Proxy ke Rust Axum)

Buka file `frontend/vite.config.ts` dan ubah konfigurasinya agar mem-proxy seluruh endpoint API ke backend Rust Axum:

```typescript
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import path from 'path';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      '$lib': path.resolve(__dirname, './src/lib'),
    },
  },
  server: {
    port: 5173,
    strictPort: true,
    // Reverse proxy API requests ke backend Axum
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:8080',
        changeOrigin: true,
      },
      '/auth': {
        target: 'http://127.0.0.1:8080',
        changeOrigin: true,
      },
      '/buyer': {
        target: 'http://127.0.0.1:8080',
        changeOrigin: true,
      },
      '/catalog': {
        target: 'http://127.0.0.1:8080',
        changeOrigin: true,
      },
      '/v1': {
        target: 'http://127.0.0.1:8080',
        changeOrigin: true,
      },
      '/uploads': {
        target: 'http://127.0.0.1:8080',
        changeOrigin: true,
      },
      '/health': {
        target: 'http://127.0.0.1:8080',
        changeOrigin: true,
      },
    },
  },
  build: {
    outDir: '../crates/web/static/dist',
    emptyOutDir: true,
  },
});
```

---

### Langkah 4: Buat Komponen Uji Sambungan `frontend/src/App.svelte`

Ganti isi file `frontend/src/App.svelte` dengan kode berikut untuk membuktikan konektivitas Svelte ke endpoint Rust:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';

  interface HealthResponse {
    status: string;
    version: string;
    subsystems?: Record<string, string>;
  }

  let healthData = $state<HealthResponse | null>(null);
  let loading = $state(true);
  let errorMsg = $state<string | null>(null);

  async function checkBackendHealth() {
    loading = true;
    errorMsg = null;
    try {
      const res = await fetch('/health');
      if (!res.ok) {
        throw new Error(`HTTP Error: ${res.status} ${res.statusText}`);
      }
      healthData = await res.json();
    } catch (err: any) {
      errorMsg = err.message || 'Gagal menghubungi server Rust Axum';
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    checkBackendHealth();
  });
</script>

<main class="container">
  <header>
    <h1>🚀 Program1 — Svelte 5 + Rust Axum</h1>
    <p class="subtitle">Modern Modular Monolith Storefront</p>
  </header>

  <section class="card">
    <h2>Pemeriksaan Koneksi Backend</h2>
    {#if loading}
      <div class="badge loading">Menghubungi server backend...</div>
    {:else if errorMsg}
      <div class="badge error">
        <strong>Error:</strong> {errorMsg}
        <p class="hint">Pastikan backend Rust sedang berjalan di port 8080: <code>cargo run -p program1-web</code></p>
      </div>
    {:else if healthData}
      <div class="badge success">
        <h3>Backend Terhubung! 🎉</h3>
        <p><strong>Status:</strong> {healthData.status}</p>
        <p><strong>Versi Server:</strong> {healthData.version}</p>
      </div>
    {/if}

    <button onclick={checkBackendHealth} class="btn-refresh">
      Cek Ulang Koneksi
    </button>
  </section>
</main>

<style>
  :global(body) {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
    background: #0f172a;
    color: #f8fafc;
    display: flex;
    justify-content: center;
    padding: 2rem;
  }
  .container {
    max-width: 600px;
    width: 100%;
    text-align: center;
  }
  header h1 {
    font-size: 1.8rem;
    color: #38bdf8;
    margin-bottom: 0.25rem;
  }
  .subtitle {
    color: #94a3b8;
    margin-top: 0;
  }
  .card {
    background: #1e293b;
    border-radius: 12px;
    padding: 1.5rem;
    margin-top: 1.5rem;
    border: 1px solid #334155;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }
  .badge {
    padding: 1rem;
    border-radius: 8px;
    margin: 1rem 0;
  }
  .badge.loading { background: #334155; color: #cbd5e1; }
  .badge.error { background: #7f1d1d; color: #fecaca; }
  .badge.success { background: #064e3b; color: #a7f3d0; text-align: left; }
  .hint { font-size: 0.85rem; margin-top: 0.5rem; opacity: 0.9; }
  .btn-refresh {
    background: #2563eb;
    color: white;
    border: none;
    padding: 0.6rem 1.2rem;
    font-size: 0.95rem;
    border-radius: 6px;
    cursor: pointer;
    font-weight: 500;
    transition: background 0.2s;
  }
  .btn-refresh:hover { background: #1d4ed8; }
</style>
```

---

### Langkah 5: Perbarui `.gitignore`

Buka file `.gitignore` di root repository `program1`, lalu tambahkan baris berikut di bagian bawah:

```gitignore
# Svelte + Bun frontend
frontend/node_modules/
frontend/dist/
crates/web/static/dist/
.bun/
```

---

### Langkah 6: Perbarui `scripts/check_dependencies.sh`

Buka file `scripts/check_dependencies.sh`, tambahkan pengecekan Bun:

```bash
# Check Bun
if command -v bun &> /dev/null; then
    echo "✓ Bun is installed: $(bun --version)"
else
    echo "⚠ Bun is not installed. Required for Svelte frontend development."
    echo "  Install via: curl -fsSL https://bun.sh/install | bash"
fi
```

---

## 🧪 Pengujian & Verifikasi

1. **Jalankan Backend Rust di Terminal 1:**
   ```bash
   cargo run -p program1-web
   ```
   Pastikan server aktif mendengarkan di port `8080` (atau port sesuai konfigurasi `.env`).

2. **Jalankan Frontend Svelte di Terminal 2:**
   ```bash
   cd frontend
   bun run dev
   ```
   Buka browser ke `http://localhost:5173`.
   - Pastikan kartu status menampilkan **"Backend Terhubung! 🎉"** dan versi aplikasi.

3. **Uji Build Produksi:**
   ```bash
   cd frontend
   bun run build
   ```
   Pastikan folder `crates/web/static/dist/` terbuat dan berisi file HTML, CSS, dan JS yang telah di-bundle.

4. **Jalankan Tes Unit Workspace Rust (Sesuai Aturan AGENTS.md):**
   ```bash
   cargo test --workspace
   ```
   Pastikan semua unit test tetap 100% lulus.

---

## ⚠️ Hal Penting & Gotchas

- **Runes di Svelte 5:** Contoh di atas menggunakan runes `$state` dan `$derived` bawaan Svelte 5. Jangan menggunakan sintaks legacy `let variable;` reaktif `$:` kecuali memang memakai Svelte 4.
- **Vite Proxy:** Saat menggunakan browser di `http://localhost:5173`, panggil URL relatif (contoh: `fetch('/health')` bukan `fetch('http://localhost:8080/health')`). Vite akan otomatis meneruskannya sehingga tidak memicu blokir CORS browser.
- **Single Source of Truth:** Jangan menduplikasi model database di frontend, cukup buat interface TypeScript yang merefleksikan DTO JSON yang dikirimkan oleh backend Rust.
