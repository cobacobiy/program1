let currentOrders = [];
let userAccounts = [];
let activeAccount = null;

const ALL_MENU_ITEMS = [
  { id: "dashboard", label: "📊 Dashboard" },
  { id: "orders", label: "📦 Orders (Pesanan)" },
  { id: "master_products", label: "🗃️ Master Products" },
  { id: "channel_products", label: "🛍️ Channel Products" },
  { id: "purchases", label: "🛒 Purchases (Pembelian)" },
  { id: "stocks", label: "🏷️ Stocks (Inventaris)" },
  { id: "warehouses", label: "🏬 Warehouses (Gudang)" },
  { id: "promotions", label: "🎟️ Promotions" },
  { id: "customers", label: "👥 Customers & CRM" },
  { id: "chat", label: "💬 Ginee Chat" },
  { id: "reports", label: "📈 Reports & Analitik" },
  { id: "logistics", label: "🚚 Logistics (Pengiriman)" },
  { id: "finances", label: "💰 Finances & Settlement" },
  { id: "integrations", label: "🌐 Integrations (Channel)" },
  { id: "settings", label: "⚙️ Settings & Hak Akses" },
  { id: "service", label: "🎧 Service & Support" }
];

async function loadData() {
  try {
    await fetchUserAccounts();
    const [analyticsRes, channelsRes, ordersRes, stocksRes, catalogRes] = await Promise.all([
      fetch("/api/v1/analytics"),
      fetch("/api/v1/channels"),
      fetch("/api/v1/orders"),
      fetch("/api/v1/inventory"),
      fetch("/api/v1/catalog")
    ]);

    const analytics = await analyticsRes.json();
    const channels = await channelsRes.json();
    currentOrders = await ordersRes.json();
    const stocks = await stocksRes.json();
    const catalog = await catalogRes.json();

    // Update Dashboard Cards
    document.getElementById("dash-revenue").innerText = `Rp ${analytics.gross_revenue.toLocaleString("id-ID")}`;
    document.getElementById("dash-orders").innerText = analytics.total_orders;
    document.getElementById("dash-products").innerText = analytics.active_products;

    renderChannelsGrid(channels);
    renderOrders(currentOrders);
    renderGineeStockList(stocks);
    renderMasterProducts(catalog);
    renderAnalyticsBreakdown(analytics.channel_breakdown);
  } catch (e) {
    console.error("Error loading OMS dashboard data:", e);
  }
}

// --- USER ACCOUNTS & RBAC PERMISSION ENGINE ---
async function fetchUserAccounts() {
  try {
    const res = await fetch("/api/v1/users/accounts");
    userAccounts = await res.json();
    if (!activeAccount && userAccounts.length > 0) {
      activeAccount = userAccounts.find(a => a.username === "admin") || userAccounts[0];
    } else if (activeAccount) {
      activeAccount = userAccounts.find(a => a.id === activeAccount.id) || userAccounts[0];
    }
    renderUserAccountSwitcher();
    renderUserAccountsTable();
    applyRBACPermissions(activeAccount);
  } catch (e) {
    console.error("Error fetching user accounts:", e);
  }
}

function renderUserAccountSwitcher() {
  if (!activeAccount) return;
  document.getElementById("header-user-avatar").innerText = activeAccount.full_name.charAt(0).toUpperCase();
  document.getElementById("header-user-name").innerText = activeAccount.full_name;
  document.getElementById("header-user-role").innerText = activeAccount.role;

  const dropdownList = document.getElementById("user-dropdown-list");
  dropdownList.innerHTML = userAccounts.map(acc => `
    <div class="dropdown-item ${acc.id === activeAccount.id ? "active" : ""}" onclick="switchActiveAccount('${acc.id}')">
      <div>
        <div style="font-size:0.8rem; font-weight:600; color:#fff">${acc.full_name}</div>
        <div style="font-size:0.7rem; color:var(--text-muted)">${acc.role}</div>
      </div>
      <span style="font-size:0.7rem; color:var(--cyan); font-weight:700">${acc.accessible_menus.length} Menu</span>
    </div>
  `).join("");
}

function toggleUserDropdown() {
  document.getElementById("user-dropdown-menu").classList.toggle("show");
}

window.onclick = function(e) {
  if (!e.target.closest(".user-switcher-container")) {
    const dropdown = document.getElementById("user-dropdown-menu");
    if (dropdown) dropdown.classList.remove("show");
  }
};

function switchActiveAccount(userId) {
  const acc = userAccounts.find(a => a.id === userId);
  if (acc) {
    activeAccount = acc;
    renderUserAccountSwitcher();
    applyRBACPermissions(activeAccount);
    document.getElementById("user-dropdown-menu").classList.remove("show");
  }
}

function applyRBACPermissions(account) {
  if (!account) return;
  const navItems = document.querySelectorAll(".nav-item[data-menu-id]");
  let activeTabVisible = false;

  navItems.forEach(item => {
    const menuId = item.getAttribute("data-menu-id");
    if (account.accessible_menus.includes(menuId)) {
      item.style.display = "flex";
      if (item.classList.contains("active")) {
        activeTabVisible = true;
      }
    } else {
      item.style.display = "none";
    }
  });

  if (!activeTabVisible && account.accessible_menus.length > 0) {
    const firstMenu = account.accessible_menus[0];
    const firstNavItem = document.querySelector(`.nav-item[data-menu-id="${firstMenu}"]`);
    if (firstNavItem) {
      switchView(firstMenu, firstNavItem);
    }
  }
}

function renderUserAccountsTable() {
  const tbody = document.getElementById("user-accounts-tbody");
  if (!tbody) return;
  tbody.innerHTML = userAccounts.map(u => `
    <tr>
      <td>
        <div style="display:flex; align-items:center; gap:0.6rem">
          <div class="user-avatar" style="width:28px; height:28px; font-size:0.75rem">${u.username.charAt(0).toUpperCase()}</div>
          <strong style="font-family:'JetBrains Mono'">${u.username}</strong>
        </div>
      </td>
      <td>${u.full_name}</td>
      <td><span class="badge-role">${u.role}</span></td>
      <td>
        <strong style="color:var(--cyan)">${u.accessible_menus.length} / ${ALL_MENU_ITEMS.length} Menu</strong>
        <div style="font-size:0.7rem; color:var(--text-muted); max-width:260px; text-overflow:ellipsis; overflow:hidden; white-space:nowrap">
          ${u.accessible_menus.join(", ")}
        </div>
      </td>
      <td><span style="color:var(--emerald); font-weight:600">● Active</span></td>
      <td>
        <button class="btn-sm btn-action" onclick="openEditPermissionsModal('${u.id}')">⚙️ Edit Hak Akses</button>
      </td>
    </tr>
  `).join("");
}

// PERMISSION MODAL HANDLERS
function openEditPermissionsModal(userId) {
  const user = userAccounts.find(u => u.id === userId);
  if (!user) return;
  document.getElementById("edit-perm-user-id").value = user.id;
  document.getElementById("edit-perm-user-label").value = `${user.full_name} (${user.role})`;

  const grid = document.getElementById("perm-checkboxes-grid");
  grid.innerHTML = ALL_MENU_ITEMS.map(m => `
    <label class="perm-item">
      <input type="checkbox" name="perm_menu" value="${m.id}" ${user.accessible_menus.includes(m.id) ? "checked" : ""}>
      <span>${m.label}</span>
    </label>
  `).join("");
  document.getElementById("edit-permissions-modal").style.display = "flex";
}

function closeEditPermissionsModal() {
  document.getElementById("edit-permissions-modal").style.display = "none";
}

function toggleAllPermCheckboxes(checked) {
  document.querySelectorAll('#perm-checkboxes-grid input[name="perm_menu"]').forEach(cb => cb.checked = checked);
}

// CREATE USER MODAL HANDLERS
function openCreateUserModal() {
  document.getElementById("create-username").value = "";
  document.getElementById("create-fullname").value = "";
  document.getElementById("create-role").value = "";
  const grid = document.getElementById("create-perm-checkboxes-grid");
  grid.innerHTML = ALL_MENU_ITEMS.map(m => `
    <label class="perm-item">
      <input type="checkbox" name="create_perm_menu" value="${m.id}" checked>
      <span>${m.label}</span>
    </label>
  `).join("");
  document.getElementById("create-user-modal").style.display = "flex";
}

function closeCreateUserModal() {
  document.getElementById("create-user-modal").style.display = "none";
}

// SUBMENU TOGGLER
function toggleSubmenu(id) {
  const el = document.getElementById(id);
  if (el) {
    el.style.display = el.style.display === "block" ? "none" : "block";
  }
}

// --- NAVIGATION & RENDERING ---
function switchView(viewName, navEl) {
  document.querySelectorAll(".nav-item").forEach(el => el.classList.remove("active"));
  if (navEl) navEl.classList.add("active");
  document.querySelectorAll(".spa-view").forEach(el => el.classList.remove("active"));

  const viewId = (viewName === "stocks" || viewName === "inventory") ? "view-stocks" : `view-${viewName}`;
  const targetView = document.getElementById(viewId);
  if (targetView) targetView.classList.add("active");

  const titleMap = {
    dashboard: { title: "Dashboard", sub: "Omnichannel Business Overview & Real-Time Analytics" },
    orders: { title: "Orders Management", sub: "Ginee OMS Centralized Order Ledger & Submenu Workflows" },
    master_products: { title: "Master Products Catalog", sub: "Global MSKU Product Information System" },
    channel_products: { title: "Channel Products Listing", sub: "Marketplace Listing & Stock Synchronization" },
    purchases: { title: "Purchases & Procurement", sub: "Supplier Purchase Orders & Automated Restock" },
    stocks: { title: "Stocks & Safety Stock", sub: "Ginee OMS Multi-Warehouse Stock & Buffer Control" },
    inventory: { title: "Stocks & Safety Stock", sub: "Ginee OMS Multi-Warehouse Stock & Buffer Control" },
    warehouses: { title: "Warehouses Hub", sub: "Multi-Warehouse Allocation & Transit Logistics" },
    promotions: { title: "Promotions & Campaign", sub: "Cross-Channel Promotion Campaign Monitor" },
    customers: { title: "Customers & CRM Directory", sub: "Buyer Profile & Loyalty Program" },
    chat: { title: "Ginee Chat Hub", sub: "Automated Multi-Channel Messaging Center" },
    reports: { title: "Reports & Financials", sub: "Financial Revenue, Sales & Performance Reports" },
    logistics: { title: "Logistics & Expedition", sub: "Order Courier Logistics Tracking" },
    finances: { title: "Finances & Settlement", sub: "Marketplace Disbursement & Revenue Reconciliation" },
    integrations: { title: "Integrations & Toko", sub: "TikTok, Shopee, Tokopedia Channel Connections" },
    settings: { title: "Settings & Hak Akses", sub: "User Account Role-Based Access Control (RBAC)" },
    service: { title: "Service & Customer Support", sub: "Ginee Customer Service Helpdesk & Ticket Logs" }
  };

  if (titleMap[viewName]) {
    document.getElementById("view-title").innerText = titleMap[viewName].title;
    document.getElementById("view-subtitle").innerText = titleMap[viewName].sub;
  }
}

function renderChannelsGrid(channels) {
  const html = channels.map(c => `
    <div style="background:rgba(255,255,255,0.03); border:1px solid var(--card-border); padding:1.2rem; border-radius:12px; display:flex; flex-direction:column; justify-content:space-between">
      <div>
        <div style="display:flex; justify-content:space-between; align-items:center">
          <span class="channel-badge ${getBadgeClass(c.channel)}">${c.name}</span>
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
  document.getElementById("dash-channels-grid").innerHTML = html;
  const pageGrid = document.getElementById("channels-page-grid");
  if (pageGrid) pageGrid.innerHTML = html;
}

function getBadgeClass(ch) {
  if (ch === "TikTokShop" || ch === "tiktok") return "badge-tiktok";
  if (ch === "Shopee" || ch === "shopee") return "badge-shopee";
  if (ch === "Tokopedia" || ch === "tokopedia") return "badge-tokopedia";
  return "badge-native";
}

function renderOrders(orders) {
  const recent = orders.slice(0, 5);
  document.getElementById("dash-orders-tbody").innerHTML = recent.length === 0 ?
    '<tr><td colspan="5" style="color:var(--text-muted)">Belum ada pesanan recorded. Place order on storefront to test!</td></tr>' :
    recent.map(o => `
      <tr>
        <td style="font-family:'JetBrains Mono'">${o.id.substring(0,8)}...</td>
        <td><span class="channel-badge ${getBadgeClass(o.channel)}">${o.channel}</span></td>
        <td>${o.customer_name}</td>
        <td style="font-weight:700; color:var(--emerald)">Rp ${o.total_amount.toLocaleString("id-ID")}</td>
        <td><strong>${o.status}</strong></td>
      </tr>
    `).join("");
  renderFullOrdersTable(orders);
}

function renderFullOrdersTable(orders) {
  const tbody = document.getElementById("orders-full-tbody");
  if (!tbody) return;
  tbody.innerHTML = orders.length === 0 ?
    '<tr><td colspan="7" style="color:var(--text-muted)">Belum ada pesanan.</td></tr>' :
    orders.map(o => `
      <tr>
        <td style="font-family:'JetBrains Mono'">${o.id.substring(0,8)}...</td>
        <td><span class="channel-badge ${getBadgeClass(o.channel)}">${o.channel}</span></td>
        <td>${o.customer_name}</td>
        <td>${o.customer_email}</td>
        <td style="font-weight:700; color:var(--emerald)">Rp ${o.total_amount.toLocaleString("id-ID")}</td>
        <td><strong>${o.status}</strong></td>
        <td style="color:var(--text-muted)">${new Date(o.created_at).toLocaleTimeString()}</td>
      </tr>
    `).join("");
}

function filterOrderTab(status, btn) {
  document.querySelectorAll("#view-orders .tab-btn").forEach(el => el.classList.remove("active"));
  if (btn) btn.classList.add("active");
  if (status === "all") {
    renderFullOrdersTable(currentOrders);
  } else {
    const filtered = currentOrders.filter(o => o.status === status);
    renderFullOrdersTable(filtered);
  }
}

function renderGineeStockList(stocks) {
  document.getElementById("ginee-stock-tbody").innerHTML = stocks.map(s => `
    <tr>
      <td>
        <div class="product-cell">
          <img src="${s.image_url}" class="product-thumb" alt="Product">
          <div class="product-meta">
            <strong>${s.product_name}</strong>
            <span>MSKU: ${s.sku}</span>
          </div>
        </div>
      </td>
      <td>Rp ${s.average_purchase_price.toLocaleString("id-ID")}</td>
      <td><strong>${s.warehouse_stock}</strong></td>
      <td>${s.spare_stock}</td>
      <td style="color:var(--rose)">${s.locked_stock}</td>
      <td>${s.promotion_stock}</td>
      <td style="font-weight:700; color:var(--emerald)">${s.available_stock}</td>
      <td style="color:var(--amber); font-weight:600">${s.safety_stock} unit</td>
      <td>
        <div style="display:flex; gap:0.4rem">
          <button class="btn-sm btn-edit-safety" onclick="openEditSafetyModal('${s.product_id}', '${s.product_name.replace(/'/g, "")}', ${s.safety_stock})">✏️ Safety Stock</button>
          <button class="btn-sm" onclick="openHistoryModal('${s.product_id}', '${s.product_name.replace(/'/g, "")}')">📜 History</button>
        </div>
      </td>
    </tr>
  `).join("");
}

function renderMasterProducts(catalog) {
  document.getElementById("master-products-tbody").innerHTML = catalog.map(p => `
    <tr>
      <td>
        <div class="product-cell">
          <img src="${p.image_url}" class="product-thumb" alt="Product">
          <div class="product-meta">
            <strong>${p.name}</strong>
            <span>${p.description || ""}</span>
          </div>
        </div>
      </td>
      <td><strong style="font-family:'JetBrains Mono'; color:var(--cyan)">${p.sku}</strong></td>
      <td>${p.category}</td>
      <td style="font-weight:700; color:var(--emerald)">Rp ${p.price.toLocaleString("id-ID")}</td>
      <td><strong>${p.stock} units</strong></td>
    </tr>
  `).join("");
}

function renderAnalyticsBreakdown(breakdown) {
  document.getElementById("analytics-breakdown-container").innerHTML = `
    <div style="display:grid; grid-template-columns:repeat(auto-fit, minmax(220px, 1fr)); gap:1.2rem">
      ${breakdown.map(b => `
        <div style="background:rgba(255,255,255,0.03); border:1px solid var(--card-border); padding:1.2rem; border-radius:12px">
          <span class="channel-badge ${getBadgeClass(b.channel)}">${b.channel_name}</span>
          <div style="font-size:1.4rem; font-weight:700; margin-top:0.6rem; color:var(--emerald)">Rp ${b.total_revenue.toLocaleString("id-ID")}</div>
          <div style="font-size:0.8rem; color:var(--text-muted); margin-top:0.2rem">Total Orders: <strong>${b.total_orders}</strong></div>
        </div>
      `).join("")}
    </div>
  `;
}

/* MODAL HANDLERS FOR SAFETY STOCK */
function openEditSafetyModal(productId, productName, currentSafety) {
  document.getElementById("edit-product-id").value = productId;
  document.getElementById("edit-product-name").value = productName;
  document.getElementById("edit-safety-val").value = currentSafety;
  document.getElementById("edit-admin-note").value = "";
  document.getElementById("edit-safety-modal").style.display = "flex";
}

function closeEditSafetyModal() {
  document.getElementById("edit-safety-modal").style.display = "none";
}

async function openHistoryModal(productId, productName) {
  document.getElementById("history-product-title").innerText = productName;
  document.getElementById("history-safety-modal").style.display = "flex";
  const res = await fetch(`/api/v1/inventory/${productId}/safety-stock-logs`);
  const logs = await res.json();
  const container = document.getElementById("history-logs-container");
  if (logs.length === 0) {
    container.innerHTML = '<p style="color:var(--text-muted)">Belum ada catatan perubahan admin untuk produk ini.</p>';
    return;
  }
  container.innerHTML = logs.map(l => `
    <div class="log-item">
      <div style="display:flex; justify-content:space-between; margin-bottom:0.3rem">
        <strong style="color:var(--amber)">Safety Stock: ${l.old_safety_stock} &rarr; ${l.new_safety_stock} unit</strong>
        <span style="font-size:0.75rem; color:var(--text-muted)">${new Date(l.timestamp).toLocaleString()}</span>
      </div>
      <p style="font-size:0.85rem; color:#fff; background:rgba(255,255,255,0.05); padding:0.5rem; border-radius:6px; margin-bottom:0.3rem">
        📝 <em>"${l.admin_note}"</em>
      </p>
      <div style="font-size:0.75rem; color:var(--cyan)">Oleh Operator: ${l.updated_by}</div>
    </div>
  `).join("");
}

function closeHistoryModal() {
  document.getElementById("history-safety-modal").style.display = "none";
}

async function syncChannel(channel) {
  await fetch(`/api/v1/channels/sync/${channel}`, { method: "POST" });
  alert(`⚡ Stock & Catalog Synced for ${channel}`);
  loadData();
}

async function syncAllChannels() {
  await fetch("/api/v1/channels/sync/tiktok", { method: "POST" });
  await fetch("/api/v1/channels/sync/shopee", { method: "POST" });
  await fetch("/api/v1/channels/sync/tokopedia", { method: "POST" });
  alert("⚡ Semua Channel (TikTok, Shopee, Tokopedia, Native Web) Berhasil Di-sync!");
  loadData();
}

document.addEventListener("DOMContentLoaded", () => {
  const editPermForm = document.getElementById("edit-permissions-form");
  if (editPermForm) {
    editPermForm.onsubmit = async (e) => {
      e.preventDefault();
      const userId = document.getElementById("edit-perm-user-id").value;
      const checked = Array.from(document.querySelectorAll('#perm-checkboxes-grid input[name="perm_menu"]:checked')).map(cb => cb.value);
      const res = await fetch(`/api/v1/users/accounts/${userId}/permissions`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ accessible_menus: checked })
      });
      if (res.ok) {
        alert("⚙️ Hak Akses User Berhasil Diperbarui!");
        closeEditPermissionsModal();
        await fetchUserAccounts();
      } else {
        const err = await res.json();
        alert(`Gagal update permissions: ${err.error || JSON.stringify(err)}`);
      }
    };
  }

  const createUserForm = document.getElementById("create-user-form");
  if (createUserForm) {
    createUserForm.onsubmit = async (e) => {
      e.preventDefault();
      const username = document.getElementById("create-username").value;
      const full_name = document.getElementById("create-fullname").value;
      const role = document.getElementById("create-role").value;
      const accessible_menus = Array.from(document.querySelectorAll('#create-perm-checkboxes-grid input[name="create_perm_menu"]:checked')).map(cb => cb.value);

      const res = await fetch("/api/v1/users/accounts", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ username, full_name, role, accessible_menus })
      });

      if (res.ok) {
        alert("➕ User Account Baru Berhasil Dibuat!");
        closeCreateUserModal();
        await fetchUserAccounts();
      } else {
        const err = await res.json();
        alert(`Gagal membuat user account: ${err.error || JSON.stringify(err)}`);
      }
    };
  }

  const editSafetyForm = document.getElementById("edit-safety-form");
  if (editSafetyForm) {
    editSafetyForm.onsubmit = async (e) => {
      e.preventDefault();
      const productId = document.getElementById("edit-product-id").value;
      const new_safety_stock = parseInt(document.getElementById("edit-safety-val").value);
      const admin_note = document.getElementById("edit-admin-note").value;
      const updated_by = document.getElementById("edit-operator").value;

      const res = await fetch(`/api/v1/inventory/${productId}/safety-stock`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ new_safety_stock, admin_note, updated_by })
      });

      if (res.ok) {
        alert("🛡️ Safety Stock & Catatan Admin Berhasil Diperbarui!");
        closeEditSafetyModal();
        loadData();
      } else {
        const err = await res.json();
        alert(`Gagal update: ${err.error || JSON.stringify(err)}`);
      }
    };
  }

  loadData();
});
