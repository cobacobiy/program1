/* ==========================================================================
   ADMIN ANALYTICS & CHANNELS ENGINE
   File: /crates/web/static/admin/admin-analytics.js
   ========================================================================== */

function getBadgeClass(ch) {
  if (ch === "TikTokShop" || ch === "tiktok") return "badge-tiktok";
  if (ch === "Shopee" || ch === "shopee") return "badge-shopee";
  if (ch === "Tokopedia" || ch === "tokopedia") return "badge-tokopedia";
  return "badge-native";
}

function renderAnalyticsBreakdown(breakdown) {
  const container = document.getElementById("analytics-breakdown-container");
  if (!container) return;

  container.innerHTML = `
    <div style="display:grid; grid-template-columns:repeat(auto-fit, minmax(220px, 1fr)); gap:1.2rem">
      ${breakdown.map(b => `
        <div style="background:rgba(255,255,255,0.03); border:1px solid var(--card-border); padding:1.2rem; border-radius:12px">
          <span class="channel-badge ${getBadgeClass(b.channel)}">${escapeHtml(b.channel_name)}</span>
          <div style="font-size:1.4rem; font-weight:700; margin-top:0.6rem; color:var(--emerald)">Rp ${b.total_revenue.toLocaleString("id-ID")}</div>
          <div style="font-size:0.8rem; color:var(--text-muted); margin-top:0.2rem">Total Orders: <strong>${b.total_orders}</strong></div>
        </div>
      `).join("")}
    </div>
  `;
}

function renderChannelsGrid(channels) {
  const dashGrid = document.getElementById("dash-channels-grid");
  const html = channels.map(c => `
    <div style="background:rgba(255,255,255,0.03); border:1px solid var(--card-border); padding:1.2rem; border-radius:12px; display:flex; flex-direction:column; justify-content:space-between">
      <div>
        <div style="display:flex; justify-content:space-between; align-items:center">
          <span class="channel-badge ${getBadgeClass(c.channel)}">${escapeHtml(c.name)}</span>
          <span style="font-size:0.75rem; color:${c.is_connected ? "var(--emerald)" : "var(--rose)"}">
            ${c.is_connected ? "● Connected" : "○ Disconnected"}
          </span>
        </div>
        <div style="font-size:1.3rem; font-weight:700; margin-top:0.8rem; color:#fff">${c.active_products_synced} Products</div>
        <div style="font-size:0.75rem; color:var(--text-muted); margin-top:0.2rem">Last Sync: ${new Date(c.last_synced_at).toLocaleTimeString()}</div>
      </div>
      <button class="btn-sm" style="margin-top:1rem; width:100%" onclick="syncChannel('${c.channel.toLowerCase()}')">⚡ Sync Now</button>
    </div>
  `).join("");

  if (dashGrid) dashGrid.innerHTML = html;
  const pageGrid = document.getElementById("channels-page-grid");
  if (pageGrid) pageGrid.innerHTML = html;
}

function renderSalesReport() {
  return `
    <div class="report-filters" style="display:flex; gap:1rem; align-items:flex-end; margin-bottom:1.5rem; flex-wrap:wrap">
      <label style="display:flex; flex-direction:column; gap:0.25rem; font-size:0.85rem">
        Dari: <input type="date" id="reportDateFrom" style="background:#1e293b; border:1px solid #334155; color:#fff; padding:0.4rem 0.6rem; border-radius:6px" />
      </label>
      <label style="display:flex; flex-direction:column; gap:0.25rem; font-size:0.85rem">
        Sampai: <input type="date" id="reportDateTo" style="background:#1e293b; border:1px solid #334155; color:#fff; padding:0.4rem 0.6rem; border-radius:6px" />
      </label>
      <label style="display:flex; flex-direction:column; gap:0.25rem; font-size:0.85rem">
        Status:
        <select id="reportStatusFilter" style="background:#1e293b; border:1px solid #334155; color:#fff; padding:0.4rem 0.6rem; border-radius:6px">
          <option value="">Semua</option>
          <option value="delivered">Delivered</option>
          <option value="shipped">Shipped</option>
          <option value="paid">Paid</option>
          <option value="pending">Pending</option>
          <option value="cancelled">Cancelled</option>
        </select>
      </label>
      <button onclick="loadSalesReport()" class="btn-sm" style="background:#0284c7; color:#fff; padding:0.5rem 1rem">Tampilkan</button>
      <button onclick="exportCSV()" class="btn-sm" style="background:#059669; color:#fff; padding:0.5rem 1rem">📥 Export CSV</button>
      <button onclick="window.print()" class="btn-sm" style="background:#475569; color:#fff; padding:0.5rem 1rem">🖨️ Cetak</button>
    </div>
    <div id="reportSummaryCards" style="display:grid; grid-template-columns:repeat(auto-fit, minmax(200px, 1fr)); gap:1rem; margin-bottom:1.5rem"></div>
    <div style="overflow-x:auto">
      <table id="reportTable" class="report-table admin-table" style="width:100%; border-collapse:collapse">
        <thead>
          <tr>
            <th>Tanggal</th>
            <th>Order ID</th>
            <th>Buyer</th>
            <th>Produk</th>
            <th>Qty</th>
            <th>Total (Rp)</th>
            <th>Status</th>
            <th>Pembayaran</th>
          </tr>
        </thead>
        <tbody id="reportBody"></tbody>
      </table>
    </div>
  `;
}

async function loadSalesReport() {
  const dateFrom = document.getElementById("reportDateFrom")?.value || "";
  const dateTo = document.getElementById("reportDateTo")?.value || "";
  const statusFilter = document.getElementById("reportStatusFilter")?.value || "";

  const params = new URLSearchParams();
  if (dateFrom) params.set("date_from", dateFrom);
  if (dateTo) params.set("date_to", dateTo);
  if (statusFilter) params.set("status_filter", statusFilter);

  const token = localStorage.getItem("program1_admin_token") || "";
  try {
    const res = await fetch(`/api/v1/analytics/report?${params.toString()}`, {
      headers: {
        Authorization: `Bearer ${token}`
      }
    });
    if (!res.ok) throw new Error("Gagal mengambil laporan");
    const data = await res.json();
    window.reportData = data;

    // Render summary
    const cardsEl = document.getElementById("reportSummaryCards");
    if (cardsEl && data.summary) {
      cardsEl.innerHTML = `
        <div class="report-summary-card" style="background:rgba(255,255,255,0.03); border:1px solid var(--card-border); padding:1rem; border-radius:8px">
          <div style="font-size:0.8rem; color:var(--text-muted)">Total Pendapatan</div>
          <div style="font-size:1.4rem; font-weight:700; color:var(--emerald)">Rp ${(data.summary.total_revenue_cents / 100).toLocaleString("id-ID")}</div>
        </div>
        <div class="report-summary-card" style="background:rgba(255,255,255,0.03); border:1px solid var(--card-border); padding:1rem; border-radius:8px">
          <div style="font-size:0.8rem; color:var(--text-muted)">Total Pesanan</div>
          <div style="font-size:1.4rem; font-weight:700; color:#fff">${data.summary.total_orders}</div>
        </div>
        <div class="report-summary-card" style="background:rgba(255,255,255,0.03); border:1px solid var(--card-border); padding:1rem; border-radius:8px">
          <div style="font-size:0.8rem; color:var(--text-muted)">Rata-Rata Order</div>
          <div style="font-size:1.4rem; font-weight:700; color:#38bdf8">Rp ${(data.summary.average_order_value_cents / 100).toLocaleString("id-ID")}</div>
        </div>
        <div class="report-summary-card" style="background:rgba(255,255,255,0.03); border:1px solid var(--card-border); padding:1rem; border-radius:8px">
          <div style="font-size:0.8rem; color:var(--text-muted)">Total Item Terjual</div>
          <div style="font-size:1.4rem; font-weight:700; color:#fbbf24">${data.summary.total_items_sold} pcs</div>
        </div>
      `;
    }

    // Render table
    const tbody = document.getElementById("reportBody");
    if (tbody) {
      if (!data.rows || data.rows.length === 0) {
        tbody.innerHTML = `<tr><td colspan="8" style="text-align:center; padding:2rem; color:var(--text-muted)">Tidak ada data untuk periode ini</td></tr>`;
      } else {
        tbody.innerHTML = data.rows.map(r => {
          const itemsStr = r.items.map(i => `${escapeHtml(i.product_name)} x${i.quantity}`).join("<br/>");
          const totalQty = r.items.reduce((s, i) => s + i.quantity, 0);
          return `
            <tr>
              <td>${new Date(r.order_date).toLocaleDateString("id-ID")}</td>
              <td><code>#${escapeHtml(r.order_id.slice(0, 8))}</code></td>
              <td>${escapeHtml(r.buyer_name)}</td>
              <td>${itemsStr}</td>
              <td style="text-align:right">${totalQty}</td>
              <td style="text-align:right">Rp ${(r.total_cents / 100).toLocaleString("id-ID")}</td>
              <td><span class="status-pill">${escapeHtml(r.status)}</span></td>
              <td><span class="status-pill">${escapeHtml(r.payment_status)}</span></td>
            </tr>
          `;
        }).join("");
      }
    }
  } catch (err) {
    alert(err.message || "Gagal memuat laporan penjualan");
  }
}

function exportCSV() {
  const rows = window.reportData?.rows || [];
  if (!rows.length) {
    alert("Tidak ada data untuk di-export");
    return;
  }

  let csv = "Tanggal,Order ID,Buyer,Email,Produk,Qty,Total (Rp),Status,Pembayaran\n";

  for (const row of rows) {
    const items = row.items.map(i => `${i.product_name} x${i.quantity}`).join("; ");
    const totalQty = row.items.reduce((s, i) => s + i.quantity, 0);
    csv += `"${row.order_date}","${row.order_id}","${row.buyer_name}","${row.buyer_email || "-"}","${items.replace(/"/g, '""')}","${totalQty}","${row.total_cents / 100}","${row.status}","${row.payment_status}"\n`;
  }

  const blob = new Blob([csv], { type: "text/csv;charset=utf-8;" });
  const link = document.createElement("a");
  link.href = URL.createObjectURL(blob);
  link.download = `laporan-penjualan-${new Date().toISOString().slice(0, 10)}.csv`;
  link.click();
}

// Window Exports
window.getBadgeClass = getBadgeClass;
window.renderAnalyticsBreakdown = renderAnalyticsBreakdown;
window.renderChannelsGrid = renderChannelsGrid;
window.renderSalesReport = renderSalesReport;
window.loadSalesReport = loadSalesReport;
window.exportCSV = exportCSV;

