<script lang="ts">
  import { cart } from '../cart.svelte';
  import { formatRupiah } from '../currency';
  import { i18n } from '../i18n.svelte';

  interface Props {
    isOpen: boolean;
    onClose: () => void;
    onStartCheckout: () => void;
  }

  let { isOpen, onClose, onStartCheckout }: Props = $props();
</script>

{#if isOpen}
  <div
    class="drawer-backdrop"
    onclick={onClose}
    role="button"
    tabindex="0"
    onkeydown={(e) => e.key === 'Escape' && onClose()}
  >
    <div class="cart-drawer" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <div class="drawer-header">
        <h3>🛒 {i18n.t('cart.title', 'Keranjang')} ({cart.totalItems})</h3>
        <button class="close-drawer" onclick={onClose}>&times;</button>
      </div>

      <div class="drawer-body">
        {#if cart.items.length === 0}
          <div class="empty-cart-state">
            <p>{i18n.t('cart.empty', 'Keranjang Anda masih kosong.')}</p>
            <button class="btn-explore" onclick={onClose}>{i18n.t('cart.continue_shopping', 'Mulai Belanja')}</button>
          </div>
        {:else}
          <div class="cart-lines">
            {#each cart.items as item (`${item.product.id}_${item.variant?.id || 'base'}`)}
              <div class="line-item">
                <div class="line-info">
                  <h4>{item.product.name}</h4>
                  {#if item.variant}
                    <span class="variant-tag">{item.variant.variant_name}: {item.variant.variant_value}</span>
                  {/if}
                  <span class="line-price">{formatRupiah(item.variant?.price_override && item.variant.price_override > 0 ? item.variant.price_override : item.product.price_cents)}</span>
                </div>
                <div class="line-actions">
                  <button class="qty-btn" onclick={() => cart.updateQuantity(item.product.id, -1, item.variant?.id)}>-</button>
                  <span class="qty-num">{item.quantity}</span>
                  <button class="qty-btn" onclick={() => cart.updateQuantity(item.product.id, 1, item.variant?.id)}>+</button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      {#if cart.items.length > 0}
        <div class="drawer-footer">
          <div class="total-bar">
            <span>{i18n.t('cart.total', 'Total')}:</span>
            <strong class="total-text">{formatRupiah(cart.totalAmountCents)}</strong>
          </div>
          <button class="btn-checkout" onclick={onStartCheckout}>
            {i18n.t('cart.checkout_btn', 'Lanjut ke Pembayaran')} 💳
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .drawer-backdrop {
    position: fixed; inset: 0; background: rgba(0,0,0,0.6); z-index: 300;
    display: flex; justify-content: flex-end;
  }
  .cart-drawer {
    width: 100%; max-width: 400px; height: 100%; background: #0f172a;
    border-left: 1px solid #1e293b; display: flex; flex-direction: column;
    box-shadow: -4px 0 20px rgba(0,0,0,0.5); animation: slideLeft 0.2s ease-out;
  }
  .drawer-header {
    padding: 1.25rem; border-bottom: 1px solid #1e293b;
    display: flex; justify-content: space-between; align-items: center;
  }
  .drawer-header h3 { margin: 0; }
  .close-drawer { background: none; border: none; font-size: 1.5rem; color: #94a3b8; cursor: pointer; }
  .drawer-body { flex: 1; overflow-y: auto; padding: 1.25rem; }
  .empty-cart-state { text-align: center; color: #94a3b8; padding: 3rem 0; }
  .btn-explore {
    background: #0284c7; color: #fff; border: none; padding: 0.5rem 1rem;
    border-radius: 6px; cursor: pointer; margin-top: 1rem;
  }
  .cart-lines { display: flex; flex-direction: column; gap: 0.75rem; }
  .line-item {
    background: #1e293b; padding: 0.75rem 1rem; border-radius: 8px; border: 1px solid #334155;
    display: flex; justify-content: space-between; align-items: center;
  }
  .line-info h4 { margin: 0 0 0.25rem; font-size: 0.95rem; }
  .line-price { color: #38bdf8; font-weight: bold; font-size: 0.9rem; }
  .line-actions { display: flex; align-items: center; gap: 0.5rem; }
  .qty-btn {
    background: #334155; color: #fff; border: none; width: 26px; height: 26px;
    border-radius: 4px; cursor: pointer; font-weight: bold;
  }
  .qty-num { min-width: 20px; text-align: center; }
  .drawer-footer { padding: 1.25rem; border-top: 1px solid #1e293b; background: #0b1120; }
  .total-bar { display: flex; justify-content: space-between; font-size: 1.1rem; margin-bottom: 1rem; }
  .total-text { color: #38bdf8; }
  .btn-checkout {
    width: 100%; background: #0284c7; color: #fff; border: none;
    padding: 0.8rem; border-radius: 8px; font-size: 1rem; font-weight: bold; cursor: pointer;
  }
  .variant-tag {
    display: inline-block; background: #0284c7; color: #fff;
    font-size: 0.72rem; padding: 0.15rem 0.45rem; border-radius: 4px;
    margin-bottom: 0.25rem; font-weight: 500;
  }
  @keyframes slideLeft {
    from { transform: translateX(100%); }
    to { transform: translateX(0); }
  }
</style>
