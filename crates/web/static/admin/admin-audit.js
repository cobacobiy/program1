/* ==========================================================================
   ADMIN AUDIT TRAIL & HISTORY MODULE
   File: /crates/web/static/admin/admin-audit.js
   ========================================================================== */

async function openUnifiedHistoryModal(productId, productName) {
  if (window.AdminState) window.AdminState.currentHistoryProductId = productId;
  window.currentHistoryProductId = productId;

  const titleEl = document.getElementById("history-stock-product-title");
  const filterEl = document.getElementById("history-filter-type");
  const modal = document.getElementById("history-stock-modal");

  if (titleEl) titleEl.innerText = productName;
  if (filterEl) filterEl.value = "";
  if (modal) modal.style.display = "flex";
  await refreshCurrentStockLogs();
}

async function refreshCurrentStockLogs() {
  const currentHistoryProductId = window.AdminState ? window.AdminState.currentHistoryProductId : window.currentHistoryProductId;
  if (!currentHistoryProductId) return;

  const filterTypeEl = document.getElementById("history-filter-type");
  const filterType = filterTypeEl ? filterTypeEl.value : "";
  const url = filterType
    ? `/api/v1/inventory/${currentHistoryProductId}/adjustment-logs?adjustment_type=${filterType}`
    : `/api/v1/inventory/${currentHistoryProductId}/adjustment-logs`;

  const container = document.getElementById("history-stock-logs-container");
  if (!container) return;
  container.innerHTML = '<p style="color:var(--text-muted)">Memuat riwayat log...</p>';

  try {
    const res = await authFetch(url);
    if (!res.ok) {
      container.innerHTML = '<p style="color:var(--rose)">Gagal memuat riwayat log.</p>';
      return;
    }
    const logs = await res.json();
    if (logs.length === 0) {
      container.innerHTML = '<p style="color:var(--text-muted); padding:1rem 0">Belum ada catatan perubahan stok untuk filter ini.</p>';
      return;
    }

    const typeIcons = {
      warehouse: "🏬 Gudang",
      safety: "🛡️ Safety Stock",
      spare: "📦 Cadangan",
      promotion: "🎟️ Promosi"
    };

    container.innerHTML = logs.map(l => {
      const typeLabel = typeIcons[l.adjustment_type] || l.adjustment_type;
      const arrow = l.new_value >= l.old_value ? "🔺" : "🔻";
      return `
        <div class="log-item">
          <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.3rem">
            <div>
              <span class="channel-badge badge-native" style="font-size:0.7rem">${typeLabel}</span>
              <strong style="color:var(--cyan); margin-left:0.4rem">${l.old_value} &rarr; ${arrow} ${l.new_value} unit</strong>
            </div>
            <span style="font-size:0.75rem; color:var(--text-muted)">${new Date(l.timestamp).toLocaleString("id-ID")}</span>
          </div>
          <p style="font-size:0.85rem; color:#fff; background:rgba(255,255,255,0.05); padding:0.5rem; border-radius:6px; margin:0.3rem 0">
            📝 <em>"${escapeHtml(l.admin_note)}"</em>
          </p>
          <div style="font-size:0.75rem; color:var(--text-muted)">Oleh Operator: <strong style="color:#fff">${escapeHtml(l.updated_by)}</strong></div>
        </div>
      `;
    }).join("");
  } catch (err) {
    container.innerHTML = `<p style="color:var(--rose)">Error: ${err.message}</p>`;
  }
}

function closeHistoryStockModal() {
  const modal = document.getElementById("history-stock-modal");
  if (modal) modal.style.display = "none";
  if (window.AdminState) window.AdminState.currentHistoryProductId = null;
  window.currentHistoryProductId = null;
}

// Backward compatibility aliases
function openHistoryModal(productId, productName) {
  openUnifiedHistoryModal(productId, productName);
}

function closeHistoryModal() {
  closeHistoryStockModal();
}

// Window Exports
window.openUnifiedHistoryModal = openUnifiedHistoryModal;
window.refreshCurrentStockLogs = refreshCurrentStockLogs;
window.closeHistoryStockModal = closeHistoryStockModal;
window.openHistoryModal = openHistoryModal;
window.closeHistoryModal = closeHistoryModal;
