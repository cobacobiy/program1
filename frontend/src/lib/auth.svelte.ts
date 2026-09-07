import { apiFetch } from './api';
import { toast } from './toast.svelte';
import type { BuyerProfile, LoginResponse } from './types';

const TOKEN_KEY = 'program1_buyer_token';

class AuthState {
  token = $state<string | null>(typeof window !== 'undefined' ? localStorage.getItem(TOKEN_KEY) : null);
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
        toast.error('Sesi login telah berakhir. Silakan login kembali.');
      });
    }
  }

  async fetchProfile() {
    try {
      this.user = await apiFetch<BuyerProfile>('/buyer/profile');
    } catch {
      this.token = null;
      this.user = null;
      if (typeof window !== 'undefined') localStorage.removeItem(TOKEN_KEY);
    }
  }

  async login(identifier: string, password: string): Promise<boolean> {
    this.isLoading = true;
    try {
      const res = await apiFetch<LoginResponse>('/auth/buyer/login', {
        method: 'POST',
        body: JSON.stringify({ identifier, password }),
      });
      this.token = res.token;
      this.user = res.buyer;
      if (typeof window !== 'undefined') localStorage.setItem(TOKEN_KEY, res.token);
      toast.success(`Selamat datang kembali, ${res.buyer.name}!`);
      this.isModalOpen = false;
      return true;
    } catch (err: any) {
      toast.error(err.message || 'Login gagal.');
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
      toast.success('Pendaftaran akun berhasil! Silakan login.');
      return true;
    } catch (err: any) {
      toast.error(err.message || 'Pendaftaran akun gagal.');
      return false;
    } finally {
      this.isLoading = false;
    }
  }

  logout(notify = true) {
    this.token = null;
    this.user = null;
    if (typeof window !== 'undefined') localStorage.removeItem(TOKEN_KEY);
    if (notify) toast.info('Anda telah keluar.');
  }
}

export const auth = new AuthState();
