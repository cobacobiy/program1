<script lang="ts">
  import { onMount } from 'svelte';
  import type { SalesReportResponse } from '../types';
  import { formatRupiah } from '../currency';
  import { toast } from '../toast.svelte';
  import { fetchAdmin } from './adminApi';
  import './admin-shared.css';

  let reportDateFrom = $state('');
  let reportDateTo = $state('');
  let reportStatusFilter = $state('all');
  let reportLoading = $state(false);
  let salesReportData = $state<SalesReportResponse | null>(null);

  async function loadSalesReport() {
    reportLoading = true;
    try {
      const params = new URLSearchParams();
      if (reportDateFrom) params.set('date_from', reportDateFrom);
      if (reportDateTo) params.set('date_to', reportDateTo);
      if (reportStatusFilter && reportStatusFilter !== 'all') {
        params.set('status_filter', reportStatusFilter);
      }
      const qs = params.toString();
      const endpoint = qs ? `/api/v1/analytics/report?${qs}` : '/api/v1/analytics/report';
      salesReportData = await fetchAdmin<SalesReportResponse>(endpoint);
    } catch (err: any) {
      toast.error(err.message || 'Gagal memuat laporan penjualan');
    } finally {
      reportLoading = false;
    }
  }

  function exportCSV() {
    const rows = salesReportData?.rows || [];
    if (!rows.length) {
      toast.error('Tidak ada data transaksi untuk diekspor.');
      return;
    }

    let csv = 'Tanggal,Order ID,Buyer,Email,Produk,Qty,Subtotal (Rp),Ongkir (Rp),Diskon (Rp),Total (Rp),Status,Status Pembayaran\n';

    for (const row of rows) {
      const itemsStr = row.items.map(i => `${i.product_name} x${i.quantity}`).join('; ');
      const totalQty = row.items.reduce((s, i) => s + i.quantity, 0);
      const subtotalRp = row.subtotal_cents / 100;
      const shippingRp = row.shipping_cents / 100;
      const discountRp = row.discount_cents / 100;
      const totalRp = row.total_cents / 100;
      const emailStr = row.buyer_email || '-';

      csv += `"${row.order_date}","${row.order_id}","${row.buyer_name}","${emailStr}","${itemsStr.replace(/"/g, '""')}","${totalQty}","${subtotalRp}","${shippingRp}","${discountRp}","${totalRp}","${row.status}","${row.payment_status}"\n`;
    }

    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const link = document.createElement('a');
    link.href = URL.createObjectURL(blob);
    const dateStr = new Date().toISOString().slice(0, 10);
    link.download = `laporan-penjualan-${dateStr}.csv`;
    link.click();
    toast.success('Laporan penjualan CSV berhasil diunduh.');
  }

  onMount(() => {
    loadSalesReport();
  });
</script>

<div class="section-panel report-panel">
  <div class="panel-top">
    <div>
      <h2>📑 Laporan Penjualan Transaksi</h2>
      <span class="panel-subtitle">Analisis detail pesanan, filter rentang tanggal, ekspor CSV, dan cetak PDF</span>
    </div>
    <div class="report-actions no-print">
      <button class="btn-action-report btn-export" onclick={exportCSV} disabled={reportLoading || !salesReportData?.rows.length}>
        📥 Export CSV
      </button>
      <button class="btn-action-report btn-print" onclick={() => window.print()} disabled={reportLoading || !salesReportData?.rows.length}>
        🖨️ Cetak / Print PDF
      </button>
    </div>
  </div>

  <!-- Filter Controls -->
  <div class="report-filter-bar no-print">
    <div class="filter-item">
      <label for="reportDateFrom">Dari Tanggal:</label>
      <input type="date" id="reportDateFrom" bind:value={reportDateFrom} class="input-date" />
    </div>
    <div class="filter-item">
      <label for="reportDateTo">Sampai Tanggal:</label>
      <input type="date" id="reportDateTo" bind:value={reportDateTo} class="input-date" />
    </div>
    <div class="filter-item">
      <label for="reportStatus">Status Order:</label>
      <select id="reportStatus" bind:value={reportStatusFilter} class="select-status">
        <option value="all">Semua Status</option>
        <option value="delivered">Delivered</option>
        <option value="shipped">Shipped</option>
        <option value="processing">Processing</option>
        <option value="paid">Paid</option>
        <option value="pending">Pending</option>
        <option value="cancelled">Cancelled</option>
      </select>
    </div>
    <button class="btn-apply-filter" onclick={loadSalesReport} disabled={reportLoading}>
      {reportLoading ? 'Memuat...' : '🔍 Tampilkan'}
    </button>
  </div>

  <!-- Summary KPI Cards -->
  {#if salesReportData}
    <div class="report-kpi-grid">
      <div class="kpi-card">
        <span class="kpi-label">Total Pendapatan</span>
        <strong class="kpi-val text-emerald">{formatRupiah(salesReportData.summary.total_revenue_cents / 100)}</strong>
        <small class="kpi-hint">Akumulasi nilai transaksi</small>
      </div>
      <div class="kpi-card">
        <span class="kpi-label">Jumlah Pesanan</span>
        <strong class="kpi-val">{salesReportData.summary.total_orders}</strong>
        <small class="kpi-hint">Total transaksi periode ini</small>
      </div>
      <div class="kpi-card">
        <span class="kpi-label">Rata-Rata Order (AOV)</span>
        <strong class="kpi-val text-cyan">{formatRupiah(salesReportData.summary.average_order_value_cents / 100)}</strong>
        <small class="kpi-hint">Rata-rata per transaksi</small>
      </div>
      <div class="kpi-card">
        <span class="kpi-label">Total Item Terjual</span>
        <strong class="kpi-val text-amber">{salesReportData.summary.total_items_sold} pcs</strong>
        <small class="kpi-hint">Kuantitas produk keluar</small>
      </div>
    </div>

    <!-- Transaction Detail Table -->
    <div class="table-container report-table-wrap">
      <table class="admin-table report-table">
        <thead>
          <tr>
            <th>Tanggal</th>
            <th>Order ID</th>
            <th>Pembeli</th>
            <th>Detail Produk</th>
            <th class="text-right">Qty</th>
            <th class="text-right">Subtotal</th>
            <th class="text-right">Ongkir</th>
            <th class="text-right">Diskon</th>
            <th class="text-right">Total Transaksi</th>
            <th>Status</th>
            <th>Pembayaran</th>
          </tr>
        </thead>
        <tbody>
          {#if salesReportData.rows.length === 0}
            <tr>
              <td colspan="11" class="empty-state">Tidak ada transaksi yang cocok dengan kriteria filter.</td>
            </tr>
          {:else}
            {#each salesReportData.rows as row}
              <tr>
                <td class="text-nowrap">{new Date(row.order_date).toLocaleDateString('id-ID', { year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })}</td>
                <td><span class="order-id-badge">#{row.order_id.slice(0, 8)}</span></td>
                <td>
                  <strong>{row.buyer_name}</strong>
                  {#if row.buyer_email}
                    <div class="text-muted">{row.buyer_email}</div>
                  {/if}
                </td>
                <td>
                  <ul class="report-item-list">
                    {#each row.items as it}
                      <li>{it.product_name} <span class="text-muted">x{it.quantity}</span></li>
                    {/each}
                  </ul>
                </td>
                <td class="text-right font-mono">{row.items.reduce((s, i) => s + i.quantity, 0)}</td>
                <td class="text-right font-mono">{formatRupiah(row.subtotal_cents / 100)}</td>
                <td class="text-right font-mono">{formatRupiah(row.shipping_cents / 100)}</td>
                <td class="text-right font-mono text-rose">-{formatRupiah(row.discount_cents / 100)}</td>
                <td class="text-right font-mono font-bold text-emerald">{formatRupiah(row.total_cents / 100)}</td>
                <td>
                  <span class="status-pill {row.status.toLowerCase()}">{row.status}</span>
                </td>
                <td>
                  <span class="status-pill payment-{row.payment_status.toLowerCase()}">{row.payment_status}</span>
                </td>
              </tr>
            {/each}
          {/if}
        </tbody>
      </table>
    </div>
  {:else if reportLoading}
    <div class="empty-state">Memuat data laporan penjualan...</div>
  {/if}
</div>

<style>
  .section-panel { width: 100%; }
  h2 { margin: 0; font-size: 1.25rem; color: #f8fafc; }
  .panel-top { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
  .panel-subtitle { font-size: 0.85rem; color: #94a3b8; margin-top: 0.25rem; display: inline-block; }
  .report-actions { display: flex; gap: 0.5rem; }
  .btn-action-report {
    border: none; padding: 0.5rem 0.9rem; border-radius: 6px; font-size: 0.85rem;
    font-weight: 600; cursor: pointer; display: flex; align-items: center; gap: 0.4rem; transition: opacity 0.2s;
  }
  .btn-export { background: #059669; color: #fff; }
  .btn-export:hover:not(:disabled) { background: #047857; }
  .btn-print { background: #475569; color: #fff; }
  .btn-print:hover:not(:disabled) { background: #334155; }
  .btn-action-report:disabled { opacity: 0.4; cursor: not-allowed; }

  .report-filter-bar {
    display: flex; gap: 1rem; align-items: flex-end; flex-wrap: wrap;
    background: #0f172a; padding: 1rem; border-radius: 8px; border: 1px solid #334155; margin-bottom: 1.5rem;
  }
  .filter-item { display: flex; flex-direction: column; gap: 0.35rem; }
  .filter-item label { font-size: 0.8rem; color: #94a3b8; font-weight: 500; }
  .input-date, .select-status {
    background: #1e293b; border: 1px solid #334155; color: #fff;
    padding: 0.45rem 0.75rem; border-radius: 6px; font-size: 0.85rem; font-family: inherit;
  }
  .btn-apply-filter {
    background: #0284c7; color: #fff; border: none; padding: 0.5rem 1.1rem;
    border-radius: 6px; font-weight: bold; cursor: pointer; transition: background 0.2s;
  }
  .btn-apply-filter:hover { background: #0369a1; }
  .btn-apply-filter:disabled { opacity: 0.5; cursor: not-allowed; }

  .report-kpi-grid {
    display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 1rem; margin-bottom: 1.5rem;
  }
  .kpi-card {
    background: #1e293b; border: 1px solid #334155; border-radius: 10px;
    padding: 1.25rem; display: flex; flex-direction: column;
  }
  .kpi-label { font-size: 0.85rem; color: #94a3b8; }
  .kpi-val { font-size: 1.6rem; margin: 0.4rem 0; font-weight: bold; }
  .kpi-hint { font-size: 0.75rem; color: #64748b; }

  .report-table-wrap { overflow-x: auto; margin-top: 0.5rem; }
  .admin-table { width: 100%; border-collapse: collapse; margin-top: 1rem; }
  .admin-table th, .admin-table td {
    padding: 0.65rem 0.85rem; text-align: left; border-bottom: 1px solid #1e293b; font-size: 0.85rem;
  }
  .admin-table th { background: #1e293b; color: #94a3b8; font-size: 0.8rem; }
  .text-right { text-align: right; }
  .text-nowrap { white-space: nowrap; }
  .text-emerald { color: #34d399; }
  .text-cyan { color: #38bdf8; }
  .text-amber { color: #fbbf24; }
  .text-rose { color: #f87171; }
  .font-mono { font-family: monospace; }
  .order-id-badge { background: #1e293b; border: 1px solid #334155; padding: 0.15rem 0.4rem; border-radius: 4px; font-family: monospace; }
  .report-item-list { list-style: none; margin: 0; padding: 0; font-size: 0.8rem; line-height: 1.35; }
  .text-muted { color: #64748b; font-size: 0.78rem; }
  .status-pill { padding: 0.2rem 0.5rem; border-radius: 4px; font-size: 0.75rem; font-weight: bold; }
  .status-pill.delivered { background: #065f46; color: #a7f3d0; }
  .status-pill.shipped { background: #7c3aed; color: #ede9fe; }
  .status-pill.processing { background: #d97706; color: #fef3c7; }
  .status-pill.paid { background: #0284c7; color: #e0f2fe; }
  .status-pill.pending { background: #854d0e; color: #fef08a; }
  .status-pill.cancelled { background: #7f1d1d; color: #fecaca; }
  .status-pill.payment-paid { background: #065f46; color: #a7f3d0; }
  .status-pill.payment-pending { background: #854d0e; color: #fef08a; }
  .status-pill.payment-cancelled { background: #7f1d1d; color: #fecaca; }
  .empty-state { text-align: center; color: #94a3b8; padding: 2rem; }

  @media print {
    :global(body) { background: #fff !important; color: #000 !important; }
    .no-print, .report-filter-bar, .report-actions, .kpi-hint { display: none !important; }
    .report-panel { background: #fff !important; color: #000 !important; border: none !important; }
    .admin-table { width: 100%; border-collapse: collapse; color: #000 !important; }
    .admin-table th, .admin-table td { border: 1px solid #ccc !important; color: #000 !important; padding: 4px 6px !important; font-size: 9pt !important; }
    .kpi-card { border: 1px solid #ccc !important; background: #fff !important; color: #000 !important; padding: 6px !important; }
    .kpi-val, .kpi-label { color: #000 !important; }
  }
</style>
