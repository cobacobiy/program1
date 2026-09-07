# Issue 12: Universal API Client Layer, Svelte Auth Store & Toast Feedback

> **Prioritas:** 🔴 CRITICAL  
> **Estimasi:** 3-4 hari  
> **Kesulitan:** ⭐⭐⭐ Menengah  
> **Prerequisite:** Issue 11 (Workspace Svelte + Bun aktif)

---

## 📋 Deskripsi

Pada frontend lama, logika autentikasi dan notifikasi tersebar di file `store-auth.js` dan `store-toast.js` dengan manipulasi DOM manual (`document.getElementById`).  
Di Issue ini, kita akan:
1. Membangun **Universal API Client** menggunakan `fetch` yang otomatis menyertakan header `Authorization: Bearer <token>` dan menangani standard format error Rust Axum (`ApiError`).
2. Membuat **Svelte Reactive State (Auth Store)** untuk login, register, verifikasi OTP telepon, dan logout.
3. Membuat komponen visual modern: **Toast Notification** dan **Auth Modal (Login/Register)** yang dipicu dari **Navbar**.

---

## 🎯 Acceptance Criteria

- [ ] Modul `apiClient` di `frontend/src/lib/api/client.ts` mendukung method GET, POST, PUT, DELETE dengan penanganan error terpusat.
- [ ] Token JWT tersimpan aman di `localStorage` dengan key `program1_buyer_token`.
- [ ] Jika server mengembalikan status `401 Unauthorized`, client otomatis membersihkan token dan mengarahkan user ke state guest.
- [ ] Komponen `ToastContainer.svelte` mampu menampilkan notifikasi `success`, `error`, dan `info` dengan animasi auto-dismiss (3 detik).
- [ ] Komponen `AuthModal.svelte` mendukung pergantian tab antara Login dan Pendaftaran Akun Pembeli baru.
- [ ] Navbar menampilkan nama profil dan tombol Keluar saat user telah login, atau tombol "Masuk / Daftar" saat guest.

---

## 📐 Langkah-Langkah Pengerjaan

### Langkah 1: Buat Tipe Data TypeScript

**File:** `frontend/src/lib/types/api.ts`

```typescript
export interface ApiSuccess<T> {
  data: T;
  message?: string;
}

export interface ApiErrorResponse {
  error_code: string;
  message: string;
  status: number;
  details?: any;
}

export interface BuyerProfile {
  id: string;
  name: string;
  email: string;
  phone: string;
  is_phone_verified: boolean;
  created_at: string;
}

export interface LoginResponse {
  token: string;
  token_type: string;
  expires_in: number;
  buyer: BuyerProfile;
}
```

---

### Langkah 2: Buat Universal API Client

**File:** `frontend/src/lib/api/client.ts`

```typescript
import type { ApiErrorResponse } from '../types/api';

const TOKEN_KEY = 'program1_buyer_token';

export class ApiError extends Error {
  errorCode: string;
  status: number;
  details?: any;

  constructor(errorResponse: ApiErrorResponse) {
    super(errorResponse.message);
    this.name = 'ApiError';
    this.errorCode = errorResponse.error_code;
    this.status = errorResponse.status;
    this.details = errorResponse.details;
  }
}

export async function apiFetch<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
  const headers = new Headers(options.headers || {});
  
  if (!headers.has('Content-Type') && !(options.body instanceof FormData)) {
    headers.set('Content-Type', 'application/json');
  }

  const token = localStorage.getItem(TOKEN_KEY);
  if (token && !headers.has('Authorization')) {
    headers.set('Authorization', `Bearer ${token}`);
  }

  const response = await fetch(endpoint, {
    ...options,
    headers,
  });

  // Tangani 401 Unauthorized (Token kadaluwarsa / invalid)
  if (response.status === 401) {
    localStorage.removeItem(TOKEN_KEY);
    window.dispatchEvent(new CustomEvent('auth:expired'));
  }

  const contentType = response.headers.get('content-type');
  const isJson = contentType && contentType.includes('application/json');
  const data = isJson ? await response.json() : await response.text();

  if (!response.ok) {
    if (isJson && data.error_code) {
      throw new ApiError(data as ApiErrorResponse);
    }
    throw new ApiError({
      error_code: 'HTTP_ERROR',
      message: typeof data === 'string' ? data : response.statusText,
      status: response.status,
    });
  }

  return data as T;
}
```

---

### Langkah 3: Buat Toast Notification Store & Component

**File:** `frontend/src/lib/stores/toast.svelte.ts`

```typescript
export interface Toast {
  id: string;
  type: 'success' | 'error' | 'info';
  message: string;
}

class ToastState {
  toasts = $state<Toast[]>([]);

  show(message: string, type: 'success' | 'error' | 'info' = 'info', durationMs = 3500) {
    const id = Math.random().toString(36).substring(2, 9);
    this.toasts = [...this.toasts, { id, type, message }];

    setTimeout(() => {
      this.remove(id);
    }, durationMs);
  }

  success(message: string) { this.show(message, 'success'); }
  error(message: string) { this.show(message, 'error'); }
  info(message: string) { this.show(message, 'info'); }

  remove(id: string) {
    this.toasts = this.toasts.filter(t => t.id !== id);
  }
}

export const toast = new ToastState();
```

**File:** `frontend/src/lib/components/ToastContainer.svelte`

```svelte
<script lang="ts">
  import { toast } from '$lib/stores/toast.svelte';
</script>

<div class="toast-wrapper">
  {#each toast.toasts as item (item.id)}
    <div class="toast-item {item.type}">
      <span>{item.message}</span>
      <button onclick={() => toast.remove(item.id)} class="btn-close">&times;</button>
    </div>
  {/each}
</div>

<style>
  .toast-wrapper {
    position: fixed;
    top: 1.5rem;
    right: 1.5rem;
    z-index: 9999;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .toast-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-width: 260px;
    max-width: 380px;
    padding: 0.8rem 1rem;
    border-radius: 8px;
    font-size: 0.9rem;
    color: #fff;
    box-shadow: 0 4px 12px rgba(0,0,0,0.3);
    animation: slideIn 0.25s ease-out;
  }
  .toast-item.success { background: #059669; }
  .toast-item.error { background: #dc2626; }
  .toast-item.info { background: #2563eb; }
  .btn-close {
    background: none;
    border: none;
    color: #fff;
    font-size: 1.2rem;
    cursor: pointer;
    margin-left: 0.75rem;
  }
  @keyframes slideIn {
    from { opacity: 0; transform: translateX(50px); }
    to { opacity: 1; transform: translateX(0); }
  }
</style>
```

---

### Langkah 4: Buat Auth Store (Svelte 5 Runes)

**File:** `frontend/src/lib/stores/auth.svelte.ts`

```typescript
import { apiFetch } from '$lib/api/client';
import { toast } from '$lib/stores/toast.svelte';
import type { BuyerProfile, LoginResponse } from '$lib/types/api';

const TOKEN_KEY = 'program1_buyer_token';

class AuthState {
  token = $state<string | null>(localStorage.getItem(TOKEN_KEY));
  user = $state<BuyerProfile | null>(null);
  isLoading = $state(false);
  isModalOpen = $state(false);

  constructor() {
    if (this.token) {
      this.fetchProfile();
    }
    if (typeof window !== 'undefined') {
      window.addEventListener('auth:expired', () => {
        this.logout(false);
        toast.error('Sesi telah berakhir, silakan login kembali.');
      });
    }
  }

  async fetchProfile() {
    try {
      this.user = await apiFetch<BuyerProfile>('/buyer/profile');
    } catch {
      this.token = null;
      this.user = null;
      localStorage.removeItem(TOKEN_KEY);
    }
  }

  async login(emailOrPhone: string, password: string): Promise<boolean> {
    this.isLoading = true;
    try {
      const res = await apiFetch<LoginResponse>('/auth/buyer/login', {
        method: 'POST',
        body: JSON.stringify({ identifier: emailOrPhone, password }),
      });
      this.token = res.token;
      this.user = res.buyer;
      localStorage.setItem(TOKEN_KEY, res.token);
      toast.success(`Selamat datang kembali, ${res.buyer.name}!`);
      this.isModalOpen = false;
      return true;
    } catch (err: any) {
      toast.error(err.message || 'Login gagal');
      return false;
    } finally {
      this.isLoading = false;
    }
  }

  async register(name: string, email: string, phone: string, password: string): Promise<boolean> {
    this.isLoading = true;
    try {
      await apiFetch('/auth/buyer/register', {
        method: 'POST',
        body: JSON.stringify({ name, email, phone, password }),
      });
      toast.success('Pendaftaran berhasil! Silakan login.');
      return true;
    } catch (err: any) {
      toast.error(err.message || 'Pendaftaran gagal');
      return false;
    } finally {
      this.isLoading = false;
    }
  }

  logout(showToast = true) {
    this.token = null;
    this.user = null;
    localStorage.removeItem(TOKEN_KEY);
    if (showToast) toast.info('Anda telah keluar.');
  }
}

export const auth = new AuthState();
```

---

### Langkah 5: Buat Komponen Navbar & Modal Autentikasi

**File:** `frontend/src/lib/components/Navbar.svelte`

```svelte
<script lang="ts">
  import { auth } from '$lib/stores/auth.svelte';
</script>

<nav class="navbar">
  <div class="nav-container">
    <div class="brand">
      <span class="logo">🛍️ Program1</span>
    </div>
    
    <div class="nav-actions">
      {#if auth.user}
        <span class="user-greeting">Halo, <strong>{auth.user.name}</strong></span>
        <button onclick={() => auth.logout()} class="btn-secondary">Keluar</button>
      {:else}
        <button onclick={() => auth.isModalOpen = true} class="btn-primary">
          Masuk / Daftar
        </button>
      {/if}
    </div>
  </div>
</nav>

<style>
  .navbar {
    background: #1e293b;
    border-bottom: 1px solid #334155;
    padding: 0.75rem 1.5rem;
  }
  .nav-container {
    max-width: 1200px;
    margin: 0 auto;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .logo { font-size: 1.25rem; font-weight: bold; color: #38bdf8; }
  .nav-actions { display: flex; align-items: center; gap: 1rem; }
  .user-greeting { font-size: 0.9rem; color: #cbd5e1; }
  .btn-primary {
    background: #0284c7; color: white; border: none;
    padding: 0.5rem 1rem; border-radius: 6px; cursor: pointer; font-weight: 500;
  }
  .btn-secondary {
    background: #475569; color: white; border: none;
    padding: 0.4rem 0.8rem; border-radius: 6px; cursor: pointer;
  }
</style>
```

**File:** `frontend/src/lib/components/AuthModal.svelte`

```svelte
<script lang="ts">
  import { auth } from '$lib/stores/auth.svelte';

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
  <div class="modal-backdrop" onclick={() => auth.isModalOpen = false}>
    <div class="modal-card" onclick={(e) => e.stopPropagation()}>
      <div class="modal-tabs">
        <button class:active={activeTab === 'login'} onclick={() => activeTab = 'login'}>Masuk</button>
        <button class:active={activeTab === 'register'} onclick={() => activeTab = 'register'}>Daftar Akun</button>
      </div>

      <form onsubmit={handleSubmit} class="auth-form">
        {#if activeTab === 'register'}
          <label>
            Nama Lengkap:
            <input type="text" bind:value={name} required placeholder="Budi Santoso" />
          </label>
          <label>
            Nomor Telepon:
            <input type="tel" bind:value={phone} required placeholder="081234567890" />
          </label>
        {/if}

        <label>
          Email / No. HP:
          <input type="text" bind:value={email} required placeholder="nama@email.com" />
        </label>

        <label>
          Password:
          <input type="password" bind:value={password} required minlength="6" />
        </label>

        <button type="submit" class="btn-submit" disabled={auth.isLoading}>
          {auth.isLoading ? 'Memproses...' : activeTab === 'login' ? 'Masuk' : 'Daftar Sekarang'}
        </button>
      </form>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed; inset: 0; background: rgba(0,0,0,0.6);
    display: flex; align-items: center; justify-content: center; z-index: 1000;
  }
  .modal-card {
    background: #1e293b; border: 1px solid #334155; border-radius: 12px;
    width: 90%; max-width: 400px; padding: 1.5rem; color: #f8fafc;
  }
  .modal-tabs { display: flex; gap: 0.5rem; margin-bottom: 1.25rem; border-bottom: 1px solid #334155; }
  .modal-tabs button {
    flex: 1; background: none; border: none; color: #94a3b8; padding: 0.6rem;
    font-size: 1rem; cursor: pointer; border-bottom: 2px solid transparent;
  }
  .modal-tabs button.active { color: #38bdf8; border-bottom-color: #38bdf8; font-weight: 600; }
  .auth-form { display: flex; flex-direction: column; gap: 0.85rem; }
  label { font-size: 0.85rem; color: #cbd5e1; display: flex; flex-direction: column; gap: 0.25rem; }
  input {
    background: #0f172a; border: 1px solid #334155; border-radius: 6px;
    padding: 0.6rem; color: #fff; font-size: 0.95rem;
  }
  .btn-submit {
    margin-top: 0.5rem; background: #0284c7; color: white; border: none;
    padding: 0.7rem; border-radius: 6px; font-weight: bold; cursor: pointer;
  }
</style>
```

---

## 🧪 Pengujian & Verifikasi

1. Buka browser ke `http://localhost:5173`.
2. Klik tombol **"Masuk / Daftar"** di Navbar.
3. Coba lakukan pendaftaran akun baru pada tab **Daftar Akun**. Pastikan toast hijau "Pendaftaran berhasil!" muncul.
4. Lakukan login dengan kredensial tersebut. Pastikan Navbar berubah menampilkan ucapan selamat datang dan tombol "Keluar".
5. Refresh browser, pastikan user tetap dalam kondisi login (sesi tersimpan via token di `localStorage`).
6. Klik tombol "Keluar", pastikan token terhapus dan kembali ke state awal.
7. Jalankan tes Rust: `cargo test --workspace` untuk menjamin tidak ada regresi di backend.

---

## ⚠️ Hal Penting & Gotchas

- **Identifier Login:** Backend `program1` mendukung login via email maupun nomor telepon Indonesia yang dinormalisasi (`08...` atau `+62...`).
- **Token Separation:** Token buyer tidak boleh digunakan untuk mengakses route admin (begitu pula sebaliknya). Di Issue 16 kita akan membuat handler khusus token admin.
