<script lang="ts">
  import { onMount } from 'svelte';
  import { apiFetch } from './api';
  import { toast } from './toast.svelte';
  import { formatRupiah } from './currency';
  import type { BuyerOrder } from './types';

  interface Props {
    isOpen: boolean;
    onClose: () => void;
  }

  let { isOpen, onClose }: Props = $props();
  let orders = $state<BuyerOrder[]>([]);
  let loading = $state(true);

  async function loadOrders() {
    loading = true;
    try {
      orders = await apiFetch<BuyerOrder[]>('/buyer/orders');
    } catch (err: any) {
      toast.error(err.message || 'Gagal memuat riwayat pesanan.');
    } finally {
      loading = false;
    }
  }

  async function confirmDelivery(orderId: string) {
    if (!confirm('Apakah pesanan ini sudah sampai dan diterima dengan baik?')) return;
    try {
      await apiFetch(`/buyer/orders/${orderId}/confirm-delivery`, { method: 'POST' });
      toast.success('Pesanan telah diselesaikan! Terima kasih.');
      await loadOrders();
    } catch (err: any) {
      toast.error(err.message || 'Gagal mengonfirmasi penerimaan.');
    }
  }

  onMount(() => {
    if (isOpen) loadOrders();
  });

  $effect(() => {
    if (isOpen) {
      loadOrders();
    }
  });
</script>

{#if isOpen}
  <div class="modal-overlay" onclick={onClose} role="button" tabindex="0" onkeydown={(e) => e.key === 'Escape' && onClose()}>
    <div class="modal-card" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <div class="modal-header">
        <h3>📦 Riwayat Pesanan Saya</h3>
        <button class="close-btn" onclick={onClose}>&times;</button>
      </div>

      <div class="orders-body">
        {#if loading}
          <p class="status-msg">Memuat daftar pesanan...</p>
        {:else if orders.length === 0}
          <div class="empty-orders">
            <p>Anda belum memiliki riwayat pesanan.</p>
          </div>
        {:else}
          <div class="orders-list">
            {#each orders as o (o.id)}
              <div class="order-box">
                <div class="box-top">
                  <div>
                    <span class="oid">#{o.id.slice(0, 8)}</span>
                    <small class="date">{new Date(o.created_at).toLocaleDateString('id-ID')}</small>
                  </div>
                  <span class="status-pill {o.status.toLowerCase()}">{o.status}</span>
                </div>

                <div class="items-summary">
                  {#each o.items as item}
                    <div class="item-line">
                      <span>{item.product_name} &times; {item.quantity}</span>
                      <span>{formatRupiah(item.price_cents * item.quantity)}</span>
                    </div>
                  {/each}
                </div>

                <div class="box-foot">
                  <div>
                    <small>Total Bayar:</small>
                    <strong class="total">{formatRupiah(o.total_amount_cents)}</strong>
                  </div>
                  {#if o.status === 'SHIPPED'}
                    <button class="btn-confirm" onclick={() => confirmDelivery(o.id)}>
                      Konfirmasi Terima
                    </button>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.7);
    display: flex; align-items: center; justify-content: center; z-index: 1000;
  }
  .modal-card {
    background: #0f172a; border: 1px solid #334155; border-radius: 12px;
    width: 90%; max-width: 600px; max-height: 85vh; display: flex; flex-direction: column;
    padding: 1.5rem; color: #f8fafc; box-shadow: 0 10px 30px rgba(0,0,0,0.5);
  }
  .modal-header {
    display: flex; justify-content: space-between; align-items: center;
    border-bottom: 1px solid #334155; padding-bottom: 0.75rem;
  }
  .modal-header h3 { margin: 0; }
  .close-btn { background: none; border: none; font-size: 1.4rem; color: #94a3b8; cursor: pointer; }
  .orders-body { flex: 1; overflow-y: auto; margin-top: 1rem; padding-right: 0.25rem; }
  .status-msg, .empty-orders { text-align: center; color: #94a3b8; padding: 2rem 0; }
  .orders-list { display: flex; flex-direction: column; gap: 0.85rem; }
  .order-box {
    background: #1e293b; border: 1px solid #334155; border-radius: 8px; padding: 1rem;
  }
  .box-top { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem; }
  .oid { font-weight: bold; color: #38bdf8; margin-right: 0.5rem; font-family: monospace; }
  .date { color: #94a3b8; }
  .status-pill {
    padding: 0.2rem 0.5rem; border-radius: 4px; font-size: 0.75rem; font-weight: bold;
  }
  .status-pill.pending_payment { background: #d97706; }
  .status-pill.paid, .status-pill.processing { background: #0284c7; }
  .status-pill.shipped { background: #7c3aed; }
  .status-pill.delivered { background: #059669; }
  .status-pill.cancelled { background: #dc2626; }
  .items-summary { border-top: 1px solid #334155; border-bottom: 1px solid #334155; padding: 0.5rem 0; margin: 0.5rem 0; }
  .item-line { display: flex; justify-content: space-between; font-size: 0.85rem; margin-bottom: 0.2rem; }
  .box-foot { display: flex; justify-content: space-between; align-items: center; }
  .total { color: #38bdf8; margin-left: 0.25rem; }
  .btn-confirm {
    background: #059669; color: white; border: none; padding: 0.4rem 0.8rem;
    border-radius: 4px; font-weight: bold; cursor: pointer; font-size: 0.85rem;
  }
</style>
