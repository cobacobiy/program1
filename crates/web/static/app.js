function escapeHtml(str) {
  if (!str) return "";
  return String(str)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

let currentOrders = [];
let userAccounts = [];
let activeAccount = null;
let authToken = localStorage.getItem("program1_jwt_token") || null;

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

async function getAuthToken(username = "admin", password = "admin123") {
  try {
    const res = await fetch("/api/v1/auth/login", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ username, password })
    });
    if (res.ok) {
      const data = await res.json();
      authToken = data.access_token;
      localStorage.setItem("program1_jwt_token", authToken);
      return authToken;
    }
  } catch (err) {
    console.error("Login authentication failed:", err);
  }
  return null;
}

async function authFetch(url, options = {}) {
  if (!authToken) {
    await getAuthToken();
  }
  options.headers = options.headers || {};
  if (authToken) {
    options.headers["Authorization"] = `Bearer ${authToken}`;
  }
  let res = await fetch(url, options);
  if (res.status === 401) {
    await getAuthToken(activeAccount ? activeAccount.username : "admin", "admin123");
    if (authToken) {
      options.headers["Authorization"] = `Bearer ${authToken}`;
      res = await fetch(url, options);
    }
  }
  return res;
}

let currentStocks = [];
let activeStockFilter = "all";
let selectedStockItem = null;
let currentHistoryProductId = null;

async function loadData() {
  try {
    await fetchUserAccounts();
    const [analyticsRes, channelsRes, ordersRes, stocksRes, catalogRes] = await Promise.all([
      authFetch("/api/v1/analytics"),
      authFetch("/api/v1/channels"),
      authFetch("/api/v1/orders"),
      authFetch("/api/v1/inventory"),
      authFetch("/api/v1/catalog")
    ]);

    if (analyticsRes.ok) {
      const analytics = await analyticsRes.json();
      document.getElementById("dash-revenue").innerText = `Rp ${analytics.gross_revenue.toLocaleString("id-ID")}`;
      document.getElementById("dash-orders").innerText = analytics.total_orders;
      document.getElementById("dash-products").innerText = analytics.active_products;
      renderAnalyticsBreakdown(analytics.channel_breakdown);
    }

    if (channelsRes.ok) {
      const channels = await channelsRes.json();
      renderChannelsGrid(channels);
    }

    if (ordersRes.ok) {
      currentOrders = await ordersRes.json();
      renderOrders(currentOrders);
    }

    if (stocksRes.ok) {
      currentStocks = await stocksRes.json();
      renderGineeStockList(currentStocks);
    }

    if (catalogRes.ok) {
      const catalog = await catalogRes.json();
      renderMasterProducts(catalog);
    }

    await loadLowStockAlerts();
  } catch (e) {
    console.error("Error loading OMS dashboard data:", e);
  }
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

// --- USER ACCOUNTS & RBAC PERMISSION ENGINE ---
async function fetchUserAccounts() {
  try {
    const res = await authFetch("/api/v1/users/accounts");
    if (res.ok) {
      userAccounts = await res.json();
      if (!activeAccount && userAccounts.length > 0) {
        activeAccount = userAccounts.find(a => a.username === "admin") || userAccounts[0];
      } else if (activeAccount) {
        activeAccount = userAccounts.find(a => a.id === activeAccount.id) || userAccounts[0];
      }
      renderUserAccountSwitcher();
      renderUserAccountsTable();
      applyRBACPermissions(activeAccount);
    }
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

async function switchActiveAccount(userId) {
  const acc = userAccounts.find(a => a.id === userId);
  if (acc) {
    activeAccount = acc;
    await getAuthToken(acc.username, "admin123");
    renderUserAccountSwitcher();
    applyRBACPermissions(activeAccount);
    document.getElementById("user-dropdown-menu").classList.remove("show");
    loadData();
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

// --- STOCK INVENTORY TABLE & FILTERING ---
const PENCIL_SVG = `<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" class="pencil-svg"><path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"></path></svg>`;

function renderGineeStockList(stocks) {
  currentStocks = stocks;
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
  activeStockFilter = filter;
  document.querySelectorAll("#view-stocks .tab-btn").forEach(el => el.classList.remove("active"));
  if (btn) {
    btn.classList.add("active");
  } else {
    const target = filter === "low" ? document.getElementById("tab-stock-low") : document.getElementById("tab-stock-all");
    if (target) target.classList.add("active");
  }
  renderGineeStockList(currentStocks);
}

function renderMasterProducts(catalog) {
  document.getElementById("master-products-tbody").innerHTML = catalog.map(p => `
    <tr>
      <td>
        <div class="product-cell">
          <img src="${escapeHtml(p.image_url)}" class="product-thumb" alt="Product" onerror="this.src='https://via.placeholder.com/40'">
          <div class="product-meta">
            <strong>${escapeHtml(p.name)}</strong>
            <span>${escapeHtml(p.description || "")}</span>
          </div>
        </div>
      </td>
      <td><strong style="font-family:'JetBrains Mono'; color:var(--cyan)">${escapeHtml(p.sku)}</strong></td>
      <td>${escapeHtml(p.category)}</td>
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
          <span class="channel-badge ${getBadgeClass(b.channel)}">${escapeHtml(b.channel_name)}</span>
          <div style="font-size:1.4rem; font-weight:700; margin-top:0.6rem; color:var(--emerald)">Rp ${b.total_revenue.toLocaleString("id-ID")}</div>
          <div style="font-size:0.8rem; color:var(--text-muted); margin-top:0.2rem">Total Orders: <strong>${b.total_orders}</strong></div>
        </div>
      `).join("")}
    </div>
  `;
}

// --- GINEE-STYLE MULTI-STOCK ADJUSTMENT MODAL ---
function openGineeStockModal(productId, stockType) {
  const stock = currentStocks.find(s => s.product_id === productId);
  if (!stock) return;

  selectedStockItem = stock;
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
  document.getElementById("stock-modal-operator").value = activeAccount ? activeAccount.full_name : "Admin Ginee";

  calculateLiveStockPreview();
  document.getElementById("edit-stock-modal").style.display = "flex";

  // Auto focus input field so user can type immediately without clicking or scrolling
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
  selectedStockItem = null;
}

function calculateLiveStockPreview() {
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

// Backward compatibility alias for any older modal calls
function openStockAdjustmentModal(productId, stockType) {
  openGineeStockModal(productId, stockType || "warehouse");
}
function openEditSafetyModal(productId, productName, currentSafety) {
  openGineeStockModal(productId, "safety");
}
function closeEditSafetyModal() {
  closeEditStockModal();
}

// --- UNIFIED HISTORY LOGS MODAL ---
async function openUnifiedHistoryModal(productId, productName) {
  currentHistoryProductId = productId;
  document.getElementById("history-stock-product-title").innerText = productName;
  document.getElementById("history-filter-type").value = "";
  document.getElementById("history-stock-modal").style.display = "flex";
  await refreshCurrentStockLogs();
}

async function refreshCurrentStockLogs() {
  if (!currentHistoryProductId) return;
  const filterType = document.getElementById("history-filter-type").value;
  const url = filterType
    ? `/api/v1/inventory/${currentHistoryProductId}/adjustment-logs?adjustment_type=${filterType}`
    : `/api/v1/inventory/${currentHistoryProductId}/adjustment-logs`;

  const container = document.getElementById("history-stock-logs-container");
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
  document.getElementById("history-stock-modal").style.display = "none";
  currentHistoryProductId = null;
}

// Backward compatibility alias
function openHistoryModal(productId, productName) {
  openUnifiedHistoryModal(productId, productName);
}
function closeHistoryModal() {
  closeHistoryStockModal();
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

  const lines = text.split("\n");
  const parsed = [];

  for (let i = 0; i < lines.length; i++) {
    const raw = lines[i].trim();
    if (!raw) continue;
    // Skip header line if detected
    if (i === 0 && (raw.toLowerCase().includes("product_id") || raw.toLowerCase().includes("sku"))) {
      continue;
    }
    const cols = raw.split(",").map(c => c.replace(/^["']|["']$/g, "").trim());
    if (cols.length >= 3) {
      // Find UUID (col 0 or scan)
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

document.addEventListener("DOMContentLoaded", () => {
  const editPermForm = document.getElementById("edit-permissions-form");
  if (editPermForm) {
    editPermForm.onsubmit = async (e) => {
      e.preventDefault();
      const userId = document.getElementById("edit-perm-user-id").value;
      const checked = Array.from(document.querySelectorAll('#perm-checkboxes-grid input[name="perm_menu"]:checked')).map(cb => cb.value);
      const res = await authFetch(`/api/v1/users/accounts/${userId}/permissions`, {
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

      const res = await authFetch("/api/v1/users/accounts", {
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

  // Multi-stock edit form handler
  const editStockForm = document.getElementById("edit-stock-form");
  if (editStockForm) {
    editStockForm.onsubmit = async (e) => {
      e.preventDefault();
      const productId = document.getElementById("stock-modal-product-id").value;
      const stockType = document.getElementById("stock-modal-type").value;
      const newVal = parseInt(document.getElementById("stock-modal-value").value);
      let admin_note = document.getElementById("stock-modal-note").value.trim();
      const updated_by = document.getElementById("stock-modal-operator").value.trim() || "Admin Ginee";

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

  loadData();
});

