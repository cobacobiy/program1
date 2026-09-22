import { adminAuth } from '../adminAuth.svelte';

export async function fetchAdmin<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
  const headers = new Headers(options.headers || {});
  if (adminAuth.token) {
    headers.set('Authorization', `Bearer ${adminAuth.token}`);
  }
  if (!headers.has('Content-Type') && !(options.body instanceof FormData)) {
    headers.set('Content-Type', 'application/json');
  }

  const res = await fetch(endpoint, { ...options, headers });
  if (res.status === 401) {
    adminAuth.logout();
    throw new Error('Sesi admin berakhir.');
  }
  const data = await res.json().catch(() => ({}));
  if (!res.ok) {
    throw new Error(data.message || `Request failed (${res.status})`);
  }
  return data as T;
}
