<script lang="ts">
  import type { Product, ProductVariant } from '../types';
  import { formatRupiah } from '../currency';
  import { i18n } from '../i18n.svelte';

  interface Props {
    product: Product | null;
    availableVariants: ProductVariant[];
    selectedVariant: ProductVariant | null;
    onSelectVariant: (v: ProductVariant) => void;
    onConfirm: () => void;
    onClose: () => void;
  }

  let {
    product,
    availableVariants,
    selectedVariant,
    onSelectVariant,
    onConfirm,
    onClose,
  }: Props = $props();
</script>

{#if product}
  <div class="drawer-backdrop" onclick={onClose} role="presentation">
    <div class="modal-card variant-picker-modal" onclick={(e) => e.stopPropagation()} role="dialog">
      <div class="modal-header">
        <div>
          <h3>{i18n.t('product.select_variant', 'Pilih Varian Produk')}</h3>
          <p class="picker-prod-name">{product.name}</p>
        </div>
        <button class="close-btn" onclick={onClose}>&times;</button>
      </div>

      <div class="picker-body">
        <label class="picker-label">{i18n.t('product.select_variant', 'Pilihan Varian')} ({product.name}):</label>
        <div class="variant-chips">
          {#each availableVariants as v (v.id)}
            <button
              class="v-chip"
              class:selected={selectedVariant?.id === v.id}
              onclick={() => onSelectVariant(v)}
            >
              <strong>{v.variant_value}</strong>
              {#if v.price_override && v.price_override > 0}
                <small class="v-price">{formatRupiah(v.price_override)}</small>
              {/if}
              <small class="v-stock">{i18n.t('product.stock_available', 'Stok')}: {v.stock_quantity}</small>
            </button>
          {/each}
        </div>

        <div class="picker-summary">
          <div class="summary-line">
            <span>{i18n.t('product.price', 'Harga')}:</span>
            <strong class="picker-price">
              {formatRupiah(selectedVariant?.price_override && selectedVariant.price_override > 0 ? selectedVariant.price_override : product.price_cents)}
            </strong>
          </div>
          <div class="summary-line">
            <span>{i18n.t('product.stock_available', 'Stok Varian')}:</span>
            <span>{selectedVariant ? selectedVariant.stock_quantity : product.stock} {i18n.t('catalog.units', 'unit')}</span>
          </div>
        </div>

        <button
          class="btn-confirm-var"
          disabled={selectedVariant ? selectedVariant.stock_quantity <= 0 : false}
          onclick={onConfirm}
        >
          {i18n.t('product.confirm_variant', '+ Masukkan ke Keranjang')}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .drawer-backdrop {
    position: fixed; inset: 0; background: rgba(0,0,0,0.6); z-index: 300;
    display: flex; align-items: center; justify-content: center;
  }
  .modal-card {
    background: #0f172a; border: 1px solid #334155; border-radius: 12px;
    box-shadow: 0 10px 30px rgba(0,0,0,0.6);
  }
  .modal-header {
    display: flex; justify-content: space-between; align-items: flex-start;
  }
  .close-btn {
    background: none; border: none; font-size: 1.5rem; color: #94a3b8; cursor: pointer;
  }
  .variant-picker-modal {
    width: 90%; max-width: 440px; padding: 1.5rem; color: #f8fafc;
  }
  .picker-prod-name { font-size: 0.85rem; color: #38bdf8; margin: 0.25rem 0 0; }
  .picker-body { margin-top: 1rem; display: flex; flex-direction: column; gap: 1rem; }
  .picker-label { font-size: 0.85rem; color: #94a3b8; }
  .variant-chips { display: flex; flex-wrap: wrap; gap: 0.6rem; }
  .v-chip {
    background: #1e293b; border: 1px solid #334155; color: #f1f5f9;
    padding: 0.5rem 0.85rem; border-radius: 8px; cursor: pointer;
    display: flex; flex-direction: column; align-items: flex-start; gap: 0.15rem;
    transition: all 0.2s;
  }
  .v-chip:hover { border-color: #38bdf8; }
  .v-chip.selected { border-color: #38bdf8; background: rgba(56, 189, 248, 0.15); box-shadow: 0 0 10px rgba(56, 189, 248, 0.2); }
  .v-price { color: #38bdf8; font-weight: bold; font-size: 0.75rem; }
  .v-stock { color: #64748b; font-size: 0.7rem; }
  .picker-summary {
    background: #1e293b; padding: 0.85rem 1rem; border-radius: 8px;
    display: flex; flex-direction: column; gap: 0.4rem; font-size: 0.9rem;
  }
  .summary-line { display: flex; justify-content: space-between; }
  .picker-price { color: #38bdf8; font-size: 1.1rem; }
  .btn-confirm-var {
    background: #0284c7; color: #fff; border: none; padding: 0.8rem;
    border-radius: 8px; font-size: 0.95rem; font-weight: bold; cursor: pointer;
  }
  .btn-confirm-var:hover { background: #0369a1; }
  .btn-confirm-var:disabled { background: #475569; cursor: not-allowed; }
</style>
