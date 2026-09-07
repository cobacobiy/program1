import type { Product, CartItem, ProductVariant } from './types';
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

  static getItemPrice(item: CartItem): number {
    if (item.variant?.price_override != null && item.variant.price_override > 0) {
      return item.variant.price_override;
    }
    return item.product.price_cents;
  }

  get totalAmountCents(): number {
    return this.items.reduce((sum, item) => sum + (CartState.getItemPrice(item) * item.quantity), 0);
  }

  addItem(product: Product, quantity = 1, variant?: ProductVariant) {
    const availableStock = variant ? variant.stock_quantity : product.stock;
    if (availableStock <= 0) {
      toast.error('Maaf, stok produk / varian ini habis.');
      return;
    }

    const idx = this.items.findIndex(i => 
      i.product.id === product.id && i.variant?.id === variant?.id
    );

    if (idx > -1) {
      const cur = this.items[idx].quantity;
      if (cur + quantity > availableStock) {
        toast.error(`Maksimal stok tersedia hanya ${availableStock} unit.`);
        return;
      }
      this.items[idx].quantity += quantity;
    } else {
      this.items = [...this.items, { product, quantity, variant }];
    }

    this.persist();
    const variantLabel = variant ? ` (${variant.variant_value})` : '';
    toast.success(`${product.name}${variantLabel} dimasukkan ke keranjang!`);
  }

  updateQuantity(productId: string, delta: number, variantId?: string) {
    const item = this.items.find(i => 
      i.product.id === productId && i.variant?.id === variantId
    );
    if (!item) return;

    const maxStock = item.variant ? item.variant.stock_quantity : item.product.stock;
    const next = item.quantity + delta;
    if (next <= 0) {
      this.removeItem(productId, variantId);
    } else if (next > maxStock) {
      toast.error(`Maksimal stok ${maxStock} unit.`);
    } else {
      item.quantity = next;
      this.persist();
    }
  }

  removeItem(productId: string, variantId?: string) {
    this.items = this.items.filter(i => 
      !(i.product.id === productId && i.variant?.id === variantId)
    );
    this.persist();
  }

  clear() {
    this.items = [];
    this.persist();
  }
}

export const cart = new CartState();
