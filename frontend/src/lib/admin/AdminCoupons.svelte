<script lang="ts">
  import type { Coupon, CreateCouponPayload, DiscountType } from '../types';
  import { formatRupiah } from '../currency';
  import { toast } from '../toast.svelte';
  import { i18n } from '../i18n.svelte';
  import { fetchAdmin } from './adminApi';

  interface Props {
    coupons: Coupon[];
    loading: boolean;
    onRefresh: () => Promise<void> | void;
  }

  let { coupons, loading, onRefresh }: Props = $props();

  let isAddCouponOpen = $state(false);
  let newCouponCode = $state('');
  let newCouponDiscountType = $state<DiscountType>('PERCENTAGE');
  let newCouponDiscountVal = $state(10);
  let newCouponMinOrder = $state(0);
  let newCouponMaxCap = $state<number | null>(null);
  let newCouponLimit = $state<number | null>(null);
  let newCouponStartDate = $state(new Date().toISOString().slice(0, 16));
  let newCouponEndDate = $state(new Date(Date.now() + 30 * 86400000).toISOString().slice(0, 16));
  let isCreatingCoupon = $state(false);

  async function handleToggleCoupon(couponId: string) {
    try {
      const updated = await fetchAdmin<Coupon>(`/api/v1/admin/coupons/${couponId}/toggle`, {
        method: 'PUT',
      });
      toast.success(`Status kupon ${updated.code} diubah!`);
      await onRefresh();
    } catch (err: any) {
      toast.error(err.message || 'Gagal mengubah status kupon');
    }
  }

  async function handleDeleteCoupon(couponId: string, code: string) {
    if (!confirm(`Hapus kupon "${code}"? Tindakan ini tidak dapat dibatalkan.`)) return;
    try {
      await fetchAdmin(`/api/v1/admin/coupons/${couponId}`, {
        method: 'DELETE',
      });
      toast.success(`Kupon "${code}" berhasil dihapus.`);
      await onRefresh();
    } catch (err: any) {
      toast.error(err.message || 'Gagal menghapus kupon');
    }
  }

  async function handleCreateCoupon(e: Event) {
    e.preventDefault();
    if (!newCouponCode.trim()) {
      toast.error('Kode kupon wajib diisi!');
      return;
    }

    isCreatingCoupon = true;
    try {
      const payload: CreateCouponPayload = {
        code: newCouponCode.trim().toUpperCase(),
        discount_type: newCouponDiscountType,
        discount_value: Number(newCouponDiscountVal),
        min_order_amount: newCouponMinOrder ? Number(newCouponMinOrder) : 0,
        max_discount_amount: newCouponMaxCap && newCouponMaxCap > 0 ? Number(newCouponMaxCap) : undefined,
        usage_limit: newCouponLimit && newCouponLimit > 0 ? Number(newCouponLimit) : undefined,
        start_date: new Date(newCouponStartDate).toISOString(),
        end_date: new Date(newCouponEndDate).toISOString(),
      };

      const created = await fetchAdmin<Coupon>('/api/v1/admin/coupons', {
        method: 'POST',
        body: JSON.stringify(payload),
      });

      toast.success(`Kupon "${created.code}" berhasil dibuat!`);
      isAddCouponOpen = false;
      newCouponCode = '';
      newCouponDiscountVal = 10;
      newCouponMaxCap = null;
      newCouponLimit = null;
      await onRefresh();
    } catch (err: any) {
      toast.error(err.message || 'Gagal membuat kupon.');
    } finally {
      isCreatingCoupon = false;
    }
  }
</script>

<div class="section-panel">
  <div class="panel-top">
    <h2>🏷️ {i18n.t('admin.coupons_mgmt', 'Manajemen Kupon & Kode Promo')}</h2>
    <button class="btn-add-prod" onclick={() => isAddCouponOpen = true}>+ {i18n.t('admin.add_coupon', 'Buat Kupon Baru')}</button>
  </div>

  {#if loading}
    <p class="empty-text">Memuat daftar kupon...</p>
  {:else if coupons.length === 0}
    <p class="empty-text">Belum ada kupon diskon. Klik tombol di atas untuk membuat kupon promo pertama!</p>
  {:else}
    <table class="table-custom">
      <thead>
        <tr>
          <th>Kode</th>
          <th>Tipe & Diskon</th>
          <th>Min. Belanja</th>
          <th>Maks. Potongan</th>
          <th>Pemakaian</th>
          <th>Periode Berlaku</th>
          <th>Status</th>
          <th>Aksi</th>
        </tr>
      </thead>
      <tbody>
        {#each coupons as c (c.id)}
          <tr>
            <td><strong class="coupon-code-badge">{c.code}</strong></td>
            <td>
              {#if c.discount_type === 'PERCENTAGE'}
                <span class="badge-tag discount-pct">{c.discount_value}% OFF</span>
              {:else}
                <span class="badge-tag discount-fix">{formatRupiah(c.discount_value)} OFF</span>
              {/if}
            </td>
            <td>{c.min_order_amount > 0 ? formatRupiah(c.min_order_amount) : 'Tanpa Min.'}</td>
            <td>{c.max_discount_amount ? formatRupiah(c.max_discount_amount) : '-'}</td>
            <td>
              <span class="usage-count">{c.usage_count}</span>
              <span class="usage-limit">/ {c.usage_limit != null ? c.usage_limit : '∞'}</span>
            </td>
            <td>
              <small>{new Date(c.start_date).toLocaleDateString('id-ID')} - {new Date(c.end_date).toLocaleDateString('id-ID')}</small>
            </td>
            <td>
              <button
                class="btn-toggle"
                class:active={c.is_active}
                onclick={() => handleToggleCoupon(c.id)}
              >
                {c.is_active ? '✓ Aktif' : '✕ Nonaktif'}
              </button>
            </td>
            <td>
              <button class="btn-var-del" onclick={() => handleDeleteCoupon(c.id, c.code)}>Hapus</button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<!-- Modal Buat Kupon Diskon -->
{#if isAddCouponOpen}
  <div class="modal-overlay" onclick={() => isAddCouponOpen = false} role="button" tabindex="0" onkeydown={(e) => e.key === 'Escape' && (isAddCouponOpen = false)}>
    <div class="modal-card" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <div class="modal-header">
        <h3>🏷️ Buat Kupon Diskon Baru</h3>
        <button class="close-btn" onclick={() => isAddCouponOpen = false}>&times;</button>
      </div>

      <form onsubmit={handleCreateCoupon} class="prod-form">
        <div class="row-fields">
          <label>
            Kode Kupon (Otomatis Kapital):
            <input type="text" bind:value={newCouponCode} required placeholder="Contoh: HEMAT20" style="text-transform: uppercase;" />
          </label>
          <label>
            Tipe Diskon:
            <select bind:value={newCouponDiscountType} class="select-custom">
              <option value="PERCENTAGE">Persentase (%)</option>
              <option value="FIXED_AMOUNT">Nominal Tetap (Rp)</option>
            </select>
          </label>
        </div>

        <div class="row-fields">
          <label>
            Nilai Diskon:
            <input
              type="number"
              bind:value={newCouponDiscountVal}
              min="1"
              max={newCouponDiscountType === 'PERCENTAGE' ? 100 : undefined}
              required
            />
          </label>
          {#if newCouponDiscountType === 'PERCENTAGE'}
            <label>
              Maksimal Potongan (Rp, Opsional):
              <input type="number" bind:value={newCouponMaxCap} placeholder="Kosongkan jika tanpa batas" min="0" />
            </label>
          {/if}
        </div>

        <div class="row-fields">
          <label>
            Min. Nilai Pesanan (Rp):
            <input type="number" bind:value={newCouponMinOrder} min="0" placeholder="0 = tanpa minimum" />
          </label>
          <label>
            Batas Total Pemakaian (Opsional):
            <input type="number" bind:value={newCouponLimit} min="1" placeholder="Kosongkan = tanpa batas" />
          </label>
        </div>

        <div class="row-fields">
          <label>
            Tanggal Mulai Berlaku:
            <input type="datetime-local" bind:value={newCouponStartDate} required />
          </label>
          <label>
            Tanggal Kedaluwarsa:
            <input type="datetime-local" bind:value={newCouponEndDate} required />
          </label>
        </div>

        <div class="modal-actions">
          <button type="button" class="btn-cancel" onclick={() => isAddCouponOpen = false}>Batal</button>
          <button type="submit" class="btn-save" disabled={isCreatingCoupon}>
            {isCreatingCoupon ? 'Membuat Kupon...' : 'Simpan Kupon 🚀'}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .section-panel { width: 100%; }
  h2 { margin: 0; font-size: 1.25rem; color: #f8fafc; }
  .panel-top { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
  .btn-add-prod {
    background: #059669; color: #fff; border: none; padding: 0.55rem 1rem;
    border-radius: 6px; font-weight: bold; cursor: pointer;
  }
  .table-custom { width: 100%; border-collapse: collapse; margin-top: 1rem; }
  .table-custom th, .table-custom td {
    padding: 0.75rem 1rem; text-align: left; border-bottom: 1px solid #1e293b; font-size: 0.9rem;
  }
  .table-custom th { background: #1e293b; color: #94a3b8; font-size: 0.8rem; }
  .coupon-code-badge {
    background: #1e3a8a; color: #93c5fd; padding: 0.2rem 0.5rem;
    border-radius: 4px; font-family: monospace; font-size: 0.85rem; letter-spacing: 0.5px;
  }
  .badge-tag { background: #334155; padding: 0.2rem 0.5rem; border-radius: 4px; font-size: 0.75rem; }
  .discount-pct { background: #065f46; color: #6ee7b7; font-weight: bold; }
  .discount-fix { background: #7c2d12; color: #fdba74; font-weight: bold; }
  .usage-count { color: #38bdf8; font-weight: bold; }
  .usage-limit { color: #64748b; font-size: 0.8rem; }
  .btn-toggle {
    border: none; padding: 0.25rem 0.6rem; border-radius: 4px;
    font-size: 0.75rem; font-weight: bold; cursor: pointer;
    background: #334155; color: #94a3b8; transition: all 0.2s;
  }
  .btn-toggle.active { background: #059669; color: #ecfdf5; }
  .btn-var-del { background: #7f1d1d; color: #fecaca; border: none; padding: 0.25rem 0.5rem; border-radius: 4px; cursor: pointer; font-size: 0.75rem; }
  .empty-text { color: #94a3b8; text-align: center; padding: 2rem 0; }

  /* Modal */
  .modal-overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.7);
    display: flex; align-items: center; justify-content: center; z-index: 1000;
  }
  .modal-card {
    background: #0f172a; border: 1px solid #334155; border-radius: 12px;
    width: 90%; max-width: 480px; padding: 1.5rem; color: #f8fafc;
  }
  .modal-header { display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid #334155; padding-bottom: 0.5rem; }
  .modal-header h3 { margin: 0; font-size: 1.15rem; color: #f8fafc; }
  .close-btn { background: none; border: none; font-size: 1.4rem; color: #94a3b8; cursor: pointer; }
  .prod-form { display: flex; flex-direction: column; gap: 0.85rem; margin-top: 1rem; }
  .prod-form label { font-size: 0.85rem; color: #cbd5e1; display: flex; flex-direction: column; gap: 0.25rem; }
  .prod-form input {
    background: #1e293b; border: 1px solid #334155; border-radius: 6px;
    padding: 0.6rem; color: #fff; font-family: inherit;
  }
  .row-fields { display: flex; gap: 1rem; }
  .row-fields label { flex: 1; }
  .select-custom {
    background: #1e293b; border: 1px solid #334155; border-radius: 6px;
    padding: 0.6rem; color: #fff; font-family: inherit;
  }
  .modal-actions { display: flex; justify-content: flex-end; gap: 0.75rem; margin-top: 0.75rem; }
  .btn-cancel { background: #334155; color: #fff; border: none; padding: 0.6rem 1.2rem; border-radius: 6px; cursor: pointer; }
  .btn-save { background: #059669; color: #fff; border: none; padding: 0.6rem 1.2rem; border-radius: 6px; font-weight: bold; cursor: pointer; }
</style>
