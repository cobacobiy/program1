import type { Product, CartItem } from './types';
import { toast } from './toast.svelte';

const STORAGE_KEY = 'program1_cart';

class CartState {
  items = $state<CartItem[]>([]);
  isOpen = $state(false);

  constructor() {
    if (typeof window !== 'undefined') {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved) {
        try {
          this.items = JSON.parse(saved);
        } catch {
          this.items = [];
        }
      }
    }
  }

  private persist() {
    if (typeof window !== 'undefined') {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.items));
    }
  }

  get totalItems(): number {
    return this.items.reduce((sum, item) => sum + item.quantity, 0);
  }

  get totalAmountCents(): number {
    return this.items.reduce((sum, item) => sum + (item.product.price_cents * item.quantity), 0);
  }

  addItem(product: Product, quantity = 1) {
    if (product.stock <= 0) {
      toast.error('Maaf, stok produk ini habis.');
      return;
    }

    const idx = this.items.findIndex(i => i.product.id === product.id);
    if (idx > -1) {
      const cur = this.items[idx].quantity;
      if (cur + quantity > product.stock) {
        toast.error(`Maksimal stok tersedia hanya ${product.stock} unit.`);
        return;
      }
      this.items[idx].quantity += quantity;
    } else {
      this.items = [...this.items, { product, quantity }];
    }

    this.persist();
    toast.success(`${product.name} dimasukkan ke keranjang!`);
  }

  updateQuantity(productId: string, delta: number) {
    const item = this.items.find(i => i.product.id === productId);
    if (!item) return;

    const next = item.quantity + delta;
    if (next <= 0) {
      this.removeItem(productId);
    } else if (next > item.product.stock) {
      toast.error(`Maksimal stok ${item.product.stock} unit.`);
    } else {
      item.quantity = next;
      this.persist();
    }
  }

  removeItem(productId: string) {
    this.items = this.items.filter(i => i.product.id !== productId);
    this.persist();
  }

  clear() {
    this.items = [];
    this.persist();
  }
}

export const cart = new CartState();
