<script lang="ts">
  import type { Product, Coupon, ProductReview } from '../types';
  import { formatRupiah } from '../currency';

  interface AdminOrder {
    id: string;
    total_amount_cents: number;
    status: string;
    created_at: string;
    tracking_number?: string | null;
  }

  interface Props {
    products: Product[];
    orders: AdminOrder[];
    coupons: Coupon[];
    adminReviews: ProductReview[];
    onOpenReports: () => void;
  }

  let { products, orders, coupons, adminReviews, onOpenReports }: Props = $props();
</script>

<div class="kpi-panel">
  <h2>📊 Ringkasan Kinerja Toko</h2>
  <div class="kpi-grid">
    <div class="kpi-card">
      <span class="kpi-label">Total Produk Aktif</span>
      <strong class="kpi-val">{products.length}</strong>
      <small class="kpi-hint">Tersedia di etalase pembeli</small>
    </div>
    <div class="kpi-card">
      <span class="kpi-label">Total Pesanan Masuk</span>
      <strong class="kpi-val">{orders.length}</strong>
      <small class="kpi-hint">Seluruh transaksi pembeli</small>
    </div>
    <div class="kpi-card">
      <span class="kpi-label">Total Omset Penjualan</span>
      <strong class="kpi-val">
        {formatRupiah(orders.reduce((sum, o) => sum + o.total_amount_cents, 0))}
      </strong>
      <small class="kpi-hint">Akumulasi nilai pesanan</small>
    </div>
    <div class="kpi-card">
      <span class="kpi-label">Kupon Promo Aktif</span>
      <strong class="kpi-val">{coupons.filter(c => c.is_active).length}</strong>
      <small class="kpi-hint">Dari total {coupons.length} voucher</small>
    </div>
    <div class="kpi-card">
      <span class="kpi-label">Ulasan Pelanggan</span>
      <strong class="kpi-val">{adminReviews.length}</strong>
      <small class="kpi-hint">{adminReviews.filter(r => r.is_visible).length} tampil publik</small>
    </div>
  </div>

  <div style="margin-top: 1.5rem; text-align: right;">
    <button class="btn-action-report btn-export" onclick={onOpenReports}>
      📑 Buka Laporan Penjualan & Ekspor Transaksi →
    </button>
  </div>
</div>

<style>
  .kpi-panel { width: 100%; }
  h2 { margin: 0 0 1.25rem; font-size: 1.25rem; color: #f8fafc; }
  .kpi-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
  }
  .kpi-card {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 10px;
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    box-shadow: 0 4px 12px rgba(0,0,0,0.2);
  }
  .kpi-label { font-size: 0.85rem; color: #94a3b8; }
  .kpi-val { font-size: 1.5rem; color: #38bdf8; font-weight: 700; }
  .kpi-hint { font-size: 0.75rem; color: #64748b; }
  .btn-action-report {
    background: #0284c7;
    color: #fff;
    border: none;
    padding: 0.6rem 1.1rem;
    border-radius: 6px;
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s;
  }
  .btn-action-report:hover { background: #0369a1; }
</style>
