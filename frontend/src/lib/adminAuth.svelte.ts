import { toast } from './toast.svelte';

const ADMIN_TOKEN_KEY = 'program1_admin_token';
const ADMIN_USER_KEY = 'program1_admin_username';

class AdminAuthState {
  token = $state<string | null>(typeof window !== 'undefined' ? localStorage.getItem(ADMIN_TOKEN_KEY) : null);
  username = $state<string | null>(typeof window !== 'undefined' ? localStorage.getItem(ADMIN_USER_KEY) : null);
  isLoading = $state(false);

  async login(user: string, pass: string): Promise<boolean> {
    this.isLoading = true;
    try {
      const res = await fetch('/auth/login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ username: user, password: pass }),
      });

      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.message || 'Login admin gagal');
      }

      const token = data.token || data.access_token;
      this.token = token;
      this.username = user;

      if (typeof window !== 'undefined') {
        localStorage.setItem(ADMIN_TOKEN_KEY, token);
        localStorage.setItem(ADMIN_USER_KEY, user);
      }

      toast.success(`Selamat datang, Admin ${user}!`);
      return true;
    } catch (err: any) {
      toast.error(err.message || 'Gagal login admin');
      return false;
    } finally {
      this.isLoading = false;
    }
  }

  logout() {
    this.token = null;
    this.username = null;
    if (typeof window !== 'undefined') {
      localStorage.removeItem(ADMIN_TOKEN_KEY);
      localStorage.removeItem(ADMIN_USER_KEY);
    }
    toast.info('Keluar dari Admin Hub.');
  }
}

export const adminAuth = new AdminAuthState();
