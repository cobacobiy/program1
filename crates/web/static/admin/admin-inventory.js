/* ==========================================================================
   ADMIN INVENTORY & STOCK MANAGEMENT ENGINE
   File: /crates/web/static/admin/admin-inventory.js
   ========================================================================== */

const PENCIL_SVG = `<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" class="pencil-svg"><path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"></path></svg>`;

function renderGineeStockList(stocks) {
  if (window.AdminState) window.AdminState.currentStocks = stocks;
  window.currentStocks = stocks;

  const activeStockFilter = window.AdminState ? window.AdminState.activeStockFilter : (window.activeStockFilter || "all");
  const filtered = activeStockFilter === "low"
    ? stocks.filter(s => s.available_stock <= s.safety_stock)
    : stocks;

  const tbody = document.getElementById("ginee-stock-tbody");
  if (!tbody) return;

  if (filtered.length === 0) {
    tbody.innerHTML = `<tr><td colspan="9" style="text-align:center; color:var(--text-muted); padding:2rem">
      ${activeStockFilter === "low" ? "✅ Tidak ada produk yang menipis! Semua stok di atas batas safety stock." : "Belum ada data inventaris."}
    </td></tr>`;
    return;
  }

  tbody.innerHTML = filtered.map(s => {
    const isLow = s.available_stock <= s.safety_stock;
    const isCritical = s.available_stock === 0 || (s.safety_stock > 0 && s.available_stock < s.safety_stock * 0.5);
    const badgeHtml = isCritical
      ? `<span class="badge-critical" style="margin-left:0.4rem">KRITIS</span>`
      : isLow
      ? `<span class="badge-warning" style="margin-left:0.4rem">MENIPIS</span>`
      : "";

    return `
      <tr>
        <td>
          <div class="product-cell">
            <img src="${escapeHtml(s.image_url)}" class="product-thumb" alt="Product" onerror="this.src='https://via.placeholder.com/40'">
            <div class="product-meta">
              <strong>${escapeHtml(s.product_name)}</strong>
              <span>MSKU: <code>${escapeHtml(s.sku)}</code></span>
            </div>
          </div>
        </td>
        <td>Rp ${s.average_purchase_price.toLocaleString("id-ID")}</td>
        <td>
          <div class="stock-editable-cell" onclick="openGineeStockModal('${escapeHtml(s.product_id)}', 'warehouse')" title="Klik untuk edit Warehouse Stock">
            <strong style="color:#fff">${s.warehouse_stock}</strong>
            <span class="btn-pencil-edit" title="Edit Warehouse Stock">${PENCIL_SVG}</span>
          </div>
        </td>
        <td>
          <div class="stock-editable-cell" onclick="openGineeStockModal('${escapeHtml(s.product_id)}', 'spare')" title="Klik untuk edit Spare Stock">
            <span style="font-weight:600">${s.spare_stock}</span>
            <span class="btn-pencil-edit" title="Edit Spare Stock">${PENCIL_SVG}</span>
          </div>
        </td>
        <td style="color:var(--rose)">${s.locked_stock}</td>
        <td>${s.promotion_stock}</td>
        <td>
          <strong style="color:var(--emerald); font-size:0.95rem">${s.available_stock}</strong>
          ${badgeHtml}
        </td>
        <td>
          <div class="stock-editable-cell" onclick="openGineeStockModal('${escapeHtml(s.product_id)}', 'safety')" title="Klik untuk edit Safety Stock">
            <span style="color:var(--amber); font-weight:600">${s.safety_stock}</span>
            <span class="btn-pencil-edit" title="Edit Safety Stock">${PENCIL_SVG}</span>
          </div>
        </td>
        <td>
          <button class="btn-sm" onclick="openUnifiedHistoryModal('${escapeHtml(s.product_id)}', '${escapeHtml(s.product_name.replace(/'/g, ''))}')">📜 Riwayat</button>
        </td>
      </tr>
    `;
  }).join("");
}

function filterStockTab(filter, btn) {
  if (window.AdminState) window.AdminState.activeStockFilter = filter;
  window.activeStockFilter = filter;

  document.querySelectorAll("#view-stocks .tab-btn").forEach(el => el.classList.remove("active"));
  if (btn) {
    btn.classList.add("active");
  } else {
    const target = filter === "low" ? document.getElementById("tab-stock-low") : document.getElementById("tab-stock-all");
    if (target) target.classList.add("active");
  }
  const currentStocks = window.AdminState ? window.AdminState.currentStocks : (window.currentStocks || []);
  renderGineeStockList(currentStocks);
}

// --- LOW STOCK ALERTS ---
async function loadLowStockAlerts() {
  const container = document.getElementById("dash-low-stock-alert");
  if (!container) return;

  try {
    const res = await authFetch("/api/v1/inventory/alerts/low-stock");
    if (!res.ok) {
      container.innerHTML = "";
      return;
    }

    const alerts = await res.json();
    if (alerts.length === 0) {
      container.innerHTML = "";
      const lowTab = document.getElementById("tab-stock-low");
      if (lowTab) lowTab.innerText = "🚨 Stok Menipis (0)";
      return;
    }

    const lowTab = document.getElementById("tab-stock-low");
    if (lowTab) lowTab.innerText = `🚨 Stok Menipis (${alerts.length})`;

    const hasCritical = alerts.some(a => a.severity === "critical");
    const alertBoxClass = hasCritical ? "alert-box alert-critical" : "alert-box";

    container.innerHTML = `
      <div class="${alertBoxClass}">
        <div class="alert-header">
          <h4>🚨 Peringatan Stok Menipis / Kritis (${alerts.length} Produk)</h4>
          <button class="btn-sm" style="background:rgba(255,255,255,0.15); color:#fff" onclick="switchView('stocks'); filterStockTab('low');">
            Lihat di Inventaris &rarr;
          </button>
        </div>
        <p style="font-size:0.8rem; color:var(--text-muted); margin-bottom:0.8rem">
          Terdapat produk dengan stok tersedia (available stock) di bawah batas safety stock yang ditentukan:
        </p>
        <ul class="alert-list">
          ${alerts.slice(0, 6).map(a => `
            <li class="alert-list-item">
              <div>
                <strong>${escapeHtml(a.product_name)}</strong>
                <div style="font-size:0.7rem; color:var(--text-muted)">MSKU: <code>${escapeHtml(a.sku)}</code></div>
              </div>
              <div style="text-align:right">
                <span class="badge-${escapeHtml(a.severity)}">${escapeHtml(a.severity.toUpperCase())}</span>
                <div style="font-size:0.75rem; margin-top:0.2rem; color:var(--amber)">
                  Tersedia: <strong>${a.available_stock}</strong> / Safety: ${a.safety_stock}
                </div>
              </div>
            </li>
          `).join("")}
        </ul>
      </div>
    `;
  } catch (err) {
    console.error("Failed to load low stock alerts:", err);
  }
}

// --- GINEE-STYLE MULTI-STOCK ADJUSTMENT MODAL ---
function openGineeStockModal(productId, stockType) {
  const currentStocks = window.AdminState ? window.AdminState.currentStocks : (window.currentStocks || []);
  const stock = currentStocks.find(s => s.product_id === productId);
  if (!stock) return;

  if (window.AdminState) window.AdminState.selectedStockItem = stock;
  window.selectedStockItem = stock;

  const targetType = stockType || "warehouse";

  document.getElementById("stock-modal-product-id").value = stock.product_id;
  document.getElementById("stock-modal-type").value = targetType;
  document.getElementById("ginee-modal-sku").innerText = stock.sku;
  document.getElementById("ginee-modal-product-name").innerText = `${stock.product_name} • Harga Modal: Rp ${stock.average_purchase_price.toLocaleString("id-ID")}`;

  const titleEl = document.getElementById("stock-modal-title");
  const currHeader = document.getElementById("ginee-table-curr-header");
  const newHeader = document.getElementById("ginee-table-new-header");
  const currValEl = document.getElementById("ginee-table-curr-val");
  const whValEl = document.getElementById("ginee-table-wh-val");
  const inputVal = document.getElementById("stock-modal-value");

  const colWhHeader = document.getElementById("ginee-col-wh-header");
  const colWhCell = document.getElementById("ginee-col-wh-cell");

  whValEl.innerText = stock.warehouse_stock;

  if (targetType === "warehouse") {
    titleEl.innerText = "Edit Warehouse Stock";
    if (colWhHeader) colWhHeader.style.display = "none";
    if (colWhCell) colWhCell.style.display = "none";
    currHeader.innerText = "Current Warehouse Stock";
    newHeader.innerText = "New Warehouse Stock";
    currValEl.innerText = stock.warehouse_stock;
    inputVal.value = stock.warehouse_stock;
  } else {
    if (colWhHeader) colWhHeader.style.display = "";
    if (colWhCell) colWhCell.style.display = "";
    if (targetType === "spare") {
      titleEl.innerText = "Edit Spare Stock";
      currHeader.innerText = "Current Spare Stock";
      newHeader.innerText = "New Spare Stock";
      currValEl.innerText = stock.spare_stock;
      inputVal.value = stock.spare_stock;
    } else if (targetType === "safety") {
      titleEl.innerText = "Edit Safety Stock (Pengingat)";
      currHeader.innerText = "Current Safety Stock";
      newHeader.innerText = "New Safety Stock";
      currValEl.innerText = stock.safety_stock;
      inputVal.value = stock.safety_stock;
    }
  }

  document.getElementById("ginee-quick-input").value = "";
  document.getElementById("stock-modal-note").value = "";
  const activeAccount = window.AdminState ? window.AdminState.activeAccount : window.activeAccount;
  const sessionOperator = activeAccount ? `${activeAccount.full_name} (${activeAccount.role})` : "Admin Super (Owner)";
  document.getElementById("stock-modal-operator").value = sessionOperator;

  calculateLiveStockPreview();
  document.getElementById("edit-stock-modal").style.display = "flex";

  setTimeout(() => {
    inputVal.focus();
    inputVal.select();
  }, 60);
}

function applyGineeQuickStock() {
  const quickVal = document.getElementById("ginee-quick-input").value;
  if (quickVal !== "") {
    document.getElementById("stock-modal-value").value = parseInt(quickVal) || 0;
    calculateLiveStockPreview();
  }
}

function closeEditStockModal() {
  document.getElementById("edit-stock-modal").style.display = "none";
  if (window.AdminState) window.AdminState.selectedStockItem = null;
  window.selectedStockItem = null;
}

function calculateLiveStockPreview() {
  const selectedStockItem = window.AdminState ? window.AdminState.selectedStockItem : window.selectedStockItem;
  if (!selectedStockItem) return;
  const type = document.getElementById("stock-modal-type").value;
  const inputVal = parseInt(document.getElementById("stock-modal-value").value) || 0;

  let wh = selectedStockItem.warehouse_stock;
  let locked = selectedStockItem.locked_stock;
  let spare = selectedStockItem.spare_stock;
  let promo = selectedStockItem.promotion_stock;

  if (type === "warehouse") wh = inputVal;
  else if (type === "spare") spare = inputVal;
  else if (type === "promotion") promo = inputVal;

  const currentAvail = Math.max(0, selectedStockItem.warehouse_stock - (selectedStockItem.spare_stock + selectedStockItem.locked_stock + selectedStockItem.promotion_stock));
  const newAvail = Math.max(0, wh - (spare + locked + promo));

  document.getElementById("stock-calc-old-avail").innerText = `${currentAvail} unit`;
  document.getElementById("stock-calc-new-avail").innerText = `${newAvail} unit`;
  document.getElementById("stock-calc-formula").innerText = `${wh} (gudang) - ${spare} (spare) - ${locked} (terkunci) - ${promo} (promosi) = ${newAvail} tersedia`;
}

// Backward compatibility aliases
function openStockAdjustmentModal(productId, stockType) {
  openGineeStockModal(productId, stockType || "warehouse");
}
function openEditSafetyModal(productId, productName, currentSafety) {
  openGineeStockModal(productId, "safety");
}
function closeEditSafetyModal() {
  closeEditStockModal();
}

// --- BULK STOCK UPDATE & CSV PARSER ---
function openBulkStockModal() {
  document.getElementById("bulk-csv-textarea").value = "";
  document.getElementById("bulk-admin-note").value = "Stock opname fisik & penyesuaian multi-channel";
  document.getElementById("bulk-preview-area").style.display = "none";
  document.getElementById("bulk-stock-modal").style.display = "flex";
}

function closeBulkStockModal() {
  document.getElementById("bulk-stock-modal").style.display = "none";
}

function downloadCsvTemplate() {
  const currentStocks = window.AdminState ? window.AdminState.currentStocks : (window.currentStocks || []);
  let csv = "product_id,product_name,sku,stock_type,new_value\n";
  currentStocks.forEach(s => {
    csv += `"${s.product_id}","${s.product_name}","${s.sku}","safety",${s.safety_stock}\n`;
  });
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8;" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.setAttribute("href", url);
  link.setAttribute("download", "template_bulk_stock_update.csv");
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
}

function handleCsvFileUpload(event) {
  const file = event.target.files[0];
  if (!file) return;
  const reader = new FileReader();
  reader.onload = function(e) {
    document.getElementById("bulk-csv-textarea").value = e.target.result;
    parseAndPreviewBulkInput();
  };
  reader.readAsText(file);
}

function parseAndPreviewBulkInput() {
  const text = document.getElementById("bulk-csv-textarea").value.trim();
  const previewArea = document.getElementById("bulk-preview-area");
  const tableContainer = document.getElementById("bulk-preview-table-container");

  if (!text) {
    previewArea.style.display = "none";
    return;
  }

  const currentStocks = window.AdminState ? window.AdminState.currentStocks : (window.currentStocks || []);
  const lines = text.split("\n");
  const parsed = [];

  for (let i = 0; i < lines.length; i++) {
    const raw = lines[i].trim();
    if (!raw) continue;
    if (i === 0 && (raw.toLowerCase().includes("product_id") || raw.toLowerCase().includes("sku"))) {
      continue;
    }
    const cols = raw.split(",").map(c => c.replace(/^["']|["']$/g, "").trim());
    if (cols.length >= 3) {
      const productId = cols[0];
      const stockType = cols.length === 3 ? cols[1] : cols[3];
      const newVal = parseInt(cols.length === 3 ? cols[2] : cols[4]);
      if (productId && stockType && !isNaN(newVal)) {
        const prod = currentStocks.find(s => s.product_id === productId);
        parsed.push({
          product_id: productId,
          product_name: prod ? prod.product_name : productId.substring(0, 8) + "...",
          stock_type: stockType,
          new_value: newVal
        });
      }
    }
  }

  if (parsed.length === 0) {
    previewArea.style.display = "none";
    return;
  }

  previewArea.style.display = "block";
  tableContainer.innerHTML = `
    <table>
      <thead>
        <tr>
          <th>Produk</th>
          <th>Tipe Stok</th>
          <th>Nilai Baru</th>
        </tr>
      </thead>
      <tbody>
        ${parsed.map(p => `
          <tr>
            <td><strong>${escapeHtml(p.product_name)}</strong></td>
            <td><code>${escapeHtml(p.stock_type)}</code></td>
            <td><strong style="color:var(--emerald)">${p.new_value} unit</strong></td>
          </tr>
        `).join("")}
      </tbody>
    </table>
  `;
}

async function submitBulkStockUpdate() {
  const text = document.getElementById("bulk-csv-textarea").value.trim();
  const admin_note = document.getElementById("bulk-admin-note").value.trim();
  if (!text) {
    alert("Silakan masukkan data baris CSV atau unggah file CSV!");
    return;
  }
  if (!admin_note) {
    alert("Catatan admin wajib diisi!");
    return;
  }

  const lines = text.split("\n");
  const adjustments = [];

  for (let i = 0; i < lines.length; i++) {
    const raw = lines[i].trim();
    if (!raw) continue;
    if (i === 0 && (raw.toLowerCase().includes("product_id") || raw.toLowerCase().includes("sku"))) {
      continue;
    }
    const cols = raw.split(",").map(c => c.replace(/^["']|["']$/g, "").trim());
    if (cols.length >= 3) {
      const product_id = cols[0];
      const stock_type = cols.length === 3 ? cols[1] : cols[3];
      const new_value = parseInt(cols.length === 3 ? cols[2] : cols[4]);
      if (product_id && stock_type && !isNaN(new_value)) {
        adjustments.push({ product_id, stock_type, new_value });
      }
    }
  }

  if (adjustments.length === 0) {
    alert("Format CSV tidak valid atau tidak ada data yang bisa diproses.");
    return;
  }

  const activeAccount = window.AdminState ? window.AdminState.activeAccount : window.activeAccount;
  const payload = {
    adjustments,
    admin_note,
    updated_by: activeAccount ? activeAccount.full_name : "Admin Ginee"
  };

  try {
    const res = await authFetch("/api/v1/inventory/bulk-update", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload)
    });

    if (res.ok) {
      const result = await res.json();
      alert(`✅ Bulk Update Selesai!\nTotal: ${result.total_requested}\nBerhasil: ${result.total_success}\nGagal: ${result.total_failed}`);
      closeBulkStockModal();
      loadData();
    } else {
      const err = await res.json();
      alert(`❌ Gagal bulk update: ${err.message || err.error || JSON.stringify(err)}`);
    }
  } catch (err) {
    alert(`❌ Gagal: ${err.message}`);
  }
}

async function syncChannel(channel) {
  await authFetch(`/api/v1/channels/sync/${channel}`, { method: "POST" });
  alert(`⚡ Stock & Catalog Synced for ${channel}`);
  loadData();
}

async function syncAllChannels() {
  await authFetch("/api/v1/channels/sync/tiktok", { method: "POST" });
  await authFetch("/api/v1/channels/sync/shopee", { method: "POST" });
  await authFetch("/api/v1/channels/sync/tokopedia", { method: "POST" });
  alert("⚡ Semua Channel (TikTok, Shopee, Tokopedia, Native Web) Berhasil Di-sync!");
  loadData();
}

// Window Exports
window.renderGineeStockList = renderGineeStockList;
window.filterStockTab = filterStockTab;
window.loadLowStockAlerts = loadLowStockAlerts;
window.openGineeStockModal = openGineeStockModal;
window.applyGineeQuickStock = applyGineeQuickStock;
window.closeEditStockModal = closeEditStockModal;
window.calculateLiveStockPreview = calculateLiveStockPreview;
window.openStockAdjustmentModal = openStockAdjustmentModal;
window.openEditSafetyModal = openEditSafetyModal;
window.closeEditSafetyModal = closeEditSafetyModal;
window.openBulkStockModal = openBulkStockModal;
window.closeBulkStockModal = closeBulkStockModal;
window.downloadCsvTemplate = downloadCsvTemplate;
window.handleCsvFileUpload = handleCsvFileUpload;
window.parseAndPreviewBulkInput = parseAndPreviewBulkInput;
window.submitBulkStockUpdate = submitBulkStockUpdate;
window.syncChannel = syncChannel;
window.syncAllChannels = syncAllChannels;

document.addEventListener("DOMContentLoaded", () => {
  const editStockForm = document.getElementById("edit-stock-form");
  if (editStockForm) {
    editStockForm.onsubmit = async (e) => {
      e.preventDefault();
      const productId = document.getElementById("stock-modal-product-id").value;
      const stockType = document.getElementById("stock-modal-type").value;
      const newVal = parseInt(document.getElementById("stock-modal-value").value);
      let admin_note = document.getElementById("stock-modal-note").value.trim();
      const activeAccount = window.AdminState ? window.AdminState.activeAccount : window.activeAccount;
      const sessionOperator = activeAccount ? `${activeAccount.full_name} (${activeAccount.role})` : "Admin Super (Owner)";
      const updated_by = document.getElementById("stock-modal-operator").value.trim() || sessionOperator;

      if (isNaN(newVal) || newVal < 0) {
        alert("Harap masukkan nilai stok yang valid (angka >= 0).");
        return;
      }

      if (!admin_note) {
        admin_note = "Penyesuaian stok manual";
      }

      let endpoint = `/api/v1/inventory/${productId}/safety-stock`;
      let payload = { admin_note, updated_by };

      if (stockType === "warehouse") {
        endpoint = `/api/v1/inventory/${productId}/warehouse-stock`;
        payload.new_warehouse_stock = newVal;
      } else if (stockType === "safety") {
        payload.new_safety_stock = newVal;
      } else if (stockType === "spare") {
        endpoint = `/api/v1/inventory/${productId}/spare-stock`;
        payload.new_spare_stock = newVal;
      } else if (stockType === "promotion") {
        endpoint = `/api/v1/inventory/${productId}/promotion-stock`;
        payload.new_promotion_stock = newVal;
      }

      try {
        const res = await authFetch(endpoint, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload)
        });

        if (res.ok) {
          alert(`✅ Stok ${stockType} Berhasil Diperbarui!`);
          closeEditStockModal();
          loadData();
        } else {
          const err = await res.json();
          const errorMsg = err.error?.message || err.message || (typeof err.error === "string" ? err.error : "") || JSON.stringify(err);
          alert(`❌ Gagal update stok: ${errorMsg}`);
        }
      } catch (err) {
        alert(`❌ Gagal: ${err.message}`);
      }
    };
  }
});
