export interface Toast {
  id: string;
  type: 'success' | 'error' | 'info';
  message: string;
}

class ToastManager {
  toasts = $state<Toast[]>([]);

  show(message: string, type: 'success' | 'error' | 'info' = 'info', durationMs = 3500) {
    const id = Math.random().toString(36).substring(2, 9);
    this.toasts = [...this.toasts, { id, type, message }];

    setTimeout(() => {
      this.remove(id);
    }, durationMs);
  }

  success(msg: string) { this.show(msg, 'success'); }
  error(msg: string) { this.show(msg, 'error'); }
  info(msg: string) { this.show(msg, 'info'); }

  remove(id: string) {
    this.toasts = this.toasts.filter(t => t.id !== id);
  }
}

export const toast = new ToastManager();
