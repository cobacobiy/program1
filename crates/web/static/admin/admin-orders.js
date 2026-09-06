/* ==========================================================================
   ADMIN ORDERS MANAGEMENT ENGINE
   File: /crates/web/static/admin/admin-orders.js
   ========================================================================== */

function getOrderStatusBadge(status) {
  const s = (status || "").toLowerCase();
  switch (s) {
    case "pending":
      return `<span class="order-status-badge status-pending">⏳ Belum Bayar</span>`;
    case "paid":
      return `<span class="order-status-badge status-paid">💳 Sudah Bayar</span>`;
    case "processing":
      return `<span class="order-status-badge status-processing">⚙️ Sedang Dikemas</span>`;
    case "shipped":
      return `<span class="order-status-badge status-shipped">🚚 Pengiriman</span>`;
    case "delivered":
      return `<span class="order-status-badge status-delivered">📬 Diterima</span>`;
    case "completed":
      return `<span class="order-status-badge status-completed">✅ Selesai</span>`;
    case "cancelled":
      return `<span class="order-status-badge status-cancelled">❌ Dibatalkan</span>`;
    case "returned":
    case "return_requested":
      return `<span class="order-status-badge status-returned">↩️ Retur</span>`;
    default:
      return `<span class="order-status-badge">${escapeHtml(status)}</span>`;
  }
}

function renderOrders(orders) {
  const dashTbody = document.getElementById("dash-orders-tbody");
  if (dashTbody) {
    const recent = orders.slice(0, 5);
    dashTbody.innerHTML = recent.length === 0 ?
      '<tr><td colspan="5" style="color:var(--text-muted)">Belum ada pesanan recorded. Place order on storefront to test!</td></tr>' :
      recent.map(o => `
        <tr>
          <td style="font-family:'JetBrains Mono'">${escapeHtml(o.id.substring(0,8))}...</td>
          <td><span class="channel-badge ${getBadgeClass(o.channel)}">${escapeHtml(o.channel)}</span></td>
          <td>${escapeHtml(o.customer_name)}</td>
          <td style="font-weight:700; color:var(--emerald)">Rp ${o.total_amount.toLocaleString("id-ID")}</td>
          <td>${getOrderStatusBadge(o.status)}</td>
        </tr>
      `).join("");
  }
  renderFullOrdersTable(orders);
}

function renderFullOrdersTable(orders) {
  const tbody = document.getElementById("orders-full-tbody");
  if (!tbody) return;
  if (orders.length === 0) {
    tbody.innerHTML = '<tr><td colspan="8" style="color:var(--text-muted); text-align:center; padding:1.5rem">Belum ada pesanan.</td></tr>';
    return;
  }

  tbody.innerHTML = orders.map(o => {
    const s = (o.status || "").toLowerCase();
    let actionHtml = `<span style="color:var(--text-muted)">-</span>`;

    if (s === "paid") {
      actionHtml = `
        <div style="display:flex; gap:0.3rem; justify-content:center">
          <button class="btn-action-sm btn-process" onclick="handleUpdateOrderStatus('${o.id}', 'processing')" title="Kemas & Proses Pesanan">⚙️ Kemas</button>
          <button class="btn-action-sm btn-cancel" onclick="handlePromptCancelOrder('${o.id}')" title="Batalkan Pesanan">❌ Batal</button>
        </div>
      `;
    } else if (s === "processing") {
      actionHtml = `
        <div style="display:flex; gap:0.3rem; justify-content:center">
          <button class="btn-action-sm btn-ship" onclick="handlePromptShipOrder('${o.id}')" title="Input Resi & Kirim">🚚 Kirim</button>
        </div>
      `;
    } else if (s === "delivered") {
      actionHtml = `
        <div style="display:flex; gap:0.3rem; justify-content:center">
          <button class="btn-action-sm btn-complete" onclick="handleUpdateOrderStatus('${o.id}', 'completed')" title="Selesaikan Pesanan">✅ Selesai</button>
        </div>
      `;
    } else if (s === "pending") {
      actionHtml = `
        <div style="display:flex; gap:0.3rem; justify-content:center">
          <button class="btn-action-sm btn-cancel" onclick="handlePromptCancelOrder('${o.id}')" title="Batalkan Pesanan">❌ Batal</button>
        </div>
      `;
    }

    let metaHtml = `<span style="color:var(--text-muted)">-</span>`;
    if (o.tracking_number) {
      metaHtml = `<code style="font-size:0.75rem; background:rgba(6,182,212,0.12); color:var(--cyan); padding:2px 6px; border-radius:4px">🚚 ${escapeHtml(o.tracking_number)}</code>`;
    } else if (o.cancel_reason) {
      metaHtml = `<span style="font-size:0.75rem; color:var(--rose)">Alasan: ${escapeHtml(o.cancel_reason.length > 20 ? o.cancel_reason.substring(0, 20) + '...' : o.cancel_reason)}</span>`;
    }

    return `
      <tr>
        <td style="font-family:'JetBrains Mono'">${escapeHtml(o.id.substring(0,8))}...</td>
        <td><span class="channel-badge ${getBadgeClass(o.channel)}">${escapeHtml(o.channel)}</span></td>
        <td>
          <strong>${escapeHtml(o.customer_name)}</strong><br>
          <small style="color:var(--text-muted); font-size:0.75rem">${escapeHtml(o.customer_email)}</small>
        </td>
        <td style="font-weight:700; color:var(--emerald)">Rp ${o.total_amount.toLocaleString("id-ID")}</td>
        <td>${getOrderStatusBadge(o.status)}</td>
        <td>${metaHtml}</td>
        <td style="color:var(--text-muted); font-size:0.8rem">${new Date(o.created_at).toLocaleString("id-ID")}</td>
        <td style="text-align:center">${actionHtml}</td>
      </tr>
    `;
  }).join("");
}

function filterOrderTab(status, btn) {
  document.querySelectorAll("#view-orders .tab-btn").forEach(el => el.classList.remove("active"));
  if (btn) btn.classList.add("active");
  const currentOrders = window.AdminState ? window.AdminState.currentOrders : (window.currentOrders || []);
  if (status === "all") {
    renderFullOrdersTable(currentOrders);
  } else {
    const filtered = currentOrders.filter(o => (o.status || "").toLowerCase() === status.toLowerCase());
    renderFullOrdersTable(filtered);
  }
}

async function handleUpdateOrderStatus(orderId, nextStatus, trackingNumber = null, cancelReason = null) {
  try {
    const payload = { status: nextStatus };
    if (trackingNumber) payload.tracking_number = trackingNumber;
    if (cancelReason) payload.cancel_reason = cancelReason;

    const res = await authFetch(`/api/v1/orders/${orderId}/status`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload)
    });

    if (res.ok) {
      alert(`✅ Status pesanan berhasil diubah menjadi '${nextStatus}'!`);
      await loadData();
    } else {
      const err = await res.json();
      alert(`❌ Gagal mengubah status pesanan: ${err.message || err.error || JSON.stringify(err)}`);
    }
  } catch (e) {
    alert(`❌ Error: ${e.message}`);
  }
}

async function handlePromptShipOrder(orderId) {
  const trackingNumber = await showAdminPrompt("Masukkan Nomor Resi / Tracking AWB Pengiriman:", "", "Kirim Pesanan", "🚚");
  if (trackingNumber !== null) {
    handleUpdateOrderStatus(orderId, "shipped", trackingNumber.trim() || null);
  }
}

async function handlePromptCancelOrder(orderId) {
  const reason = await showAdminPrompt("Masukkan alasan pembatalan pesanan:", "Dibatalkan oleh admin toko", "Batalkan Pesanan", "❌");
  if (reason !== null) {
    handleUpdateOrderStatus(orderId, "cancelled", null, reason.trim() || "Dibatalkan oleh admin toko");
  }
}

// Window Exports
window.getOrderStatusBadge = getOrderStatusBadge;
window.renderOrders = renderOrders;
window.renderFullOrdersTable = renderFullOrdersTable;
window.filterOrderTab = filterOrderTab;
window.handleUpdateOrderStatus = handleUpdateOrderStatus;
window.handlePromptShipOrder = handlePromptShipOrder;
window.handlePromptCancelOrder = handlePromptCancelOrder;
