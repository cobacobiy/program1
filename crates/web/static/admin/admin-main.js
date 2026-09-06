/* ==========================================================================
   ADMIN PORTAL MAIN ENGINE & SHARED STATE
   File: /crates/web/static/admin/admin-main.js
   ========================================================================== */

function escapeHtml(str) {
  if (!str) return "";
  return String(str)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

// In-App Toast notification system for admin panel (eliminates browser native IP popups)
function showAdminToast(message, type = "info", duration = 4000) {
  let container = document.getElementById("admin-toast-container") || document.getElementById("store-toast-container");
  if (!container) {
    container = document.createElement("div");
    container.id = "admin-toast-container";
    container.className = "store-toast-container";
    document.body.appendChild(container);
  }

  const toast = document.createElement("div");
  toast.className = `store-toast ${type}`;

  let icon = "ℹ️";
  if (type === "success") icon = "✅";
  if (type === "warning") icon = "⚠️";
  if (type === "error") icon = "❌";

  toast.innerHTML = `<span>${icon}</span><div style="flex:1">${message}</div>`;
  container.appendChild(toast);

  setTimeout(() => {
    toast.style.opacity = "0";
    toast.style.transform = "translateX(100%)";
    setTimeout(() => toast.remove(), 300);
  }, duration);
}

// Override alert for admin panel so IP address never appears in native browser dialogs
window.alert = function(msg) {
  let t = "info";
  const str = String(msg);
  if (str.includes("🎉") || str.includes("✅") || str.includes("berhasil") || str.includes("Berhasil")) {
    t = "success";
  } else if (str.includes("Gagal") || str.includes("Error") || str.includes("error") || str.includes("❌")) {
    t = "error";
  } else if (str.includes("Harap") || str.includes("Silakan") || str.includes("⚠️") || str.includes("Ditolak")) {
    t = "warning";
  }
  showAdminToast(str, t);
};

// --- MODERN IN-APP ADMIN CONFIRM & PROMPT MODALS ---
function ensureAdminDialogModal() {
  let modal = document.getElementById("admin-dialog-modal");
  if (!modal) {
    modal = document.createElement("div");
    modal.id = "admin-dialog-modal";
    modal.className = "modal-overlay";
    modal.style.zIndex = "9999";
    modal.innerHTML = `
      <div class="modal-card" style="max-width: 420px; text-align: center; padding: 1.75rem; border-radius: 12px; background: var(--bg-card, #1e293b); border: 1px solid var(--border-color, #334155); box-shadow: 0 20px 25px -5px rgba(0,0,0,0.5);">
        <div id="admin-dialog-icon" style="font-size: 2.2rem; margin-bottom: 0.5rem;">⚠️</div>
        <h3 id="admin-dialog-title" style="margin: 0 0 0.5rem; font-size: 1.15rem; font-weight: 700; color: var(--text-heading, #f8fafc);">Konfirmasi</h3>
        <p id="admin-dialog-message" style="font-size: 0.9rem; color: var(--text-muted, #94a3b8); margin-bottom: 1.25rem; line-height: 1.45;"></p>
        <div id="admin-dialog-input-wrap" style="display: none; margin-bottom: 1.25rem;">
          <input type="text" id="admin-dialog-input" class="form-control" style="width: 100%; box-sizing: border-box; padding: 0.65rem 0.85rem; border-radius: 6px; border: 1px solid var(--border-color, #475569); background: var(--bg-input, #0f172a); color: #fff; font-size: 0.9rem;">
        </div>
        <div style="display: flex; gap: 0.75rem; justify-content: center;">
          <button type="button" id="admin-dialog-cancel" class="btn-action" style="padding: 0.6rem 1.25rem; min-width: 95px; border-radius: 6px; cursor: pointer;">Batal</button>
          <button type="button" id="admin-dialog-confirm" class="btn-action btn-primary" style="padding: 0.6rem 1.25rem; min-width: 95px; border-radius: 6px; cursor: pointer;">Lanjutkan</button>
        </div>
      </div>
    `;
    document.body.appendChild(modal);
  }
  return modal;
}

let adminDialogResolver = null;

function showAdminConfirm(message, title = "Konfirmasi Tindakan", icon = "⚠️") {
  return new Promise((resolve) => {
    const modal = ensureAdminDialogModal();
    adminDialogResolver = resolve;
    document.getElementById("admin-dialog-icon").textContent = icon;
    document.getElementById("admin-dialog-title").textContent = title;
    document.getElementById("admin-dialog-message").textContent = message;
    document.getElementById("admin-dialog-input-wrap").style.display = "none";

    const cancelBtn = document.getElementById("admin-dialog-cancel");
    const confirmBtn = document.getElementById("admin-dialog-confirm");

    cancelBtn.onclick = () => {
      modal.style.display = "none";
      if (adminDialogResolver) {
        adminDialogResolver(false);
        adminDialogResolver = null;
      }
    };
    confirmBtn.onclick = () => {
      modal.style.display = "none";
      if (adminDialogResolver) {
        adminDialogResolver(true);
        adminDialogResolver = null;
      }
    };
    modal.style.display = "flex";
  });
}

function showAdminPrompt(message, defaultValue = "", title = "Input Diperlukan", icon = "📝") {
  return new Promise((resolve) => {
    const modal = ensureAdminDialogModal();
    adminDialogResolver = resolve;
    document.getElementById("admin-dialog-icon").textContent = icon;
    document.getElementById("admin-dialog-title").textContent = title;
    document.getElementById("admin-dialog-message").textContent = message;

    const inputWrap = document.getElementById("admin-dialog-input-wrap");
    const inputEl = document.getElementById("admin-dialog-input");
    inputWrap.style.display = "block";
    inputEl.value = defaultValue;

    const cancelBtn = document.getElementById("admin-dialog-cancel");
    const confirmBtn = document.getElementById("admin-dialog-confirm");

    const cleanup = () => {
      inputEl.onkeydown = null;
    };

    cancelBtn.onclick = () => {
      cleanup();
      modal.style.display = "none";
      if (adminDialogResolver) {
        adminDialogResolver(null);
        adminDialogResolver = null;
      }
    };
    confirmBtn.onclick = () => {
      cleanup();
      modal.style.display = "none";
      if (adminDialogResolver) {
        adminDialogResolver(inputEl.value);
        adminDialogResolver = null;
      }
    };
    inputEl.onkeydown = (e) => {
      if (e.key === "Enter") {
        e.preventDefault();
        confirmBtn.click();
      } else if (e.key === "Escape") {
        e.preventDefault();
        cancelBtn.click();
      }
    };

    modal.style.display = "flex";
    setTimeout(() => inputEl.focus(), 50);
  });
}

// Override native confirm & prompt as safety nets to eliminate "192.168.6.xxx says:" browser popups
window.confirm = function(msg) {
  console.warn("Native confirm blocked to prevent IP dialog:", msg);
  return true;
};
window.prompt = function(msg, defaultText) {
  console.warn("Native prompt blocked to prevent IP dialog:", msg);
  return defaultText || "";
};

// --- GLOBAL ADMIN STATE ---
const AdminState = {
  authToken: localStorage.getItem("program1_seller_jwt_token") || localStorage.getItem("program1_jwt_token") || null,
  activeAccount: null,
  userAccounts: [],
  currentOrders: [],
  currentStocks: [],
  activeStockFilter: "all",
  selectedStockItem: null,
  currentHistoryProductId: null,
  currentCustomers: [],
  currentCustomerTab: "list"
};

window.AdminState = AdminState;

// Bind legacy globals to AdminState
["authToken", "activeAccount", "userAccounts", "currentOrders", "currentStocks",
 "activeStockFilter", "selectedStockItem", "currentHistoryProductId",
 "currentCustomers", "currentCustomerTab"].forEach(prop => {
  Object.defineProperty(window, prop, {
    get() { return AdminState[prop]; },
    set(val) { AdminState[prop] = val; },
    configurable: true
  });
});

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

window.ALL_MENU_ITEMS = ALL_MENU_ITEMS;

// --- ADMIN THEME ENGINE (DARK / LIGHT) ---
function initAdminTheme() {
  const savedTheme = localStorage.getItem("program1_admin_theme") || "dark";
  applyAdminTheme(savedTheme);
}

function applyAdminTheme(theme) {
  document.documentElement.setAttribute("data-theme", theme);
  localStorage.setItem("program1_admin_theme", theme);
  const icons = document.querySelectorAll(".admin-theme-icon");
  icons.forEach(ic => {
    ic.innerText = theme === "light" ? "🌙" : "☀️";
  });
}

function toggleAdminTheme() {
  const currentTheme = document.documentElement.getAttribute("data-theme") || "dark";
  const newTheme = currentTheme === "light" ? "dark" : "light";
  applyAdminTheme(newTheme);
}

window.toggleAdminTheme = toggleAdminTheme;
initAdminTheme();

// --- SELLER PORTAL AUTHENTICATION & LOGIN ENGINE ---
function showSellerLoginScreen() {
  const loginScreen = document.getElementById("seller-login-screen");
  const adminApp = document.getElementById("admin-main-app");
  if (loginScreen) loginScreen.style.display = "flex";
  if (adminApp) adminApp.style.display = "none";
  const errAlert = document.getElementById("login-error-alert");
  if (errAlert) {
    errAlert.style.display = "none";
    errAlert.innerText = "";
  }
  const uInput = document.getElementById("login-username");
  if (uInput) setTimeout(() => uInput.focus(), 50);
}

function showAdminMainApp() {
  const loginScreen = document.getElementById("seller-login-screen");
  const adminApp = document.getElementById("admin-main-app");
  if (loginScreen) loginScreen.style.display = "none";
  if (adminApp) adminApp.style.display = "flex";
}

async function loginAdmin(username, password) {
  const errAlert = document.getElementById("login-error-alert");
  const submitBtn = document.getElementById("btn-submit-seller-login");
  if (errAlert) {
    errAlert.style.display = "none";
    errAlert.innerText = "";
  }
  if (submitBtn) {
    submitBtn.disabled = true;
    submitBtn.innerText = "⏳ Memverifikasi Akun...";
  }

  try {
    const res = await fetch("/api/v1/auth/login", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ username, password })
    });

    if (res.ok) {
      const data = await res.json();
      AdminState.authToken = data.access_token;
      window.authToken = data.access_token;
      localStorage.setItem("program1_seller_jwt_token", data.access_token);
      localStorage.setItem("program1_seller_active_user", username);
      showAdminMainApp();
      await loadData();
      return true;
    } else {
      const err = await res.json().catch(() => ({}));
      const msg = err.error || err.message || "Username atau password salah. Silakan coba lagi.";
      if (errAlert) {
        errAlert.innerText = `❌ Login Gagal: ${msg}`;
        errAlert.style.display = "block";
      }
      return false;
    }
  } catch (e) {
    if (errAlert) {
      errAlert.innerText = `❌ Kesalahan koneksi: ${e.message}`;
      errAlert.style.display = "block";
    }
    return false;
  } finally {
    if (submitBtn) {
      submitBtn.disabled = false;
      submitBtn.innerText = "🚀 Masuk ke Seller Portal";
    }
  }
}

function handleSellerLogin(event) {
  if (event) event.preventDefault();
  const uInput = document.getElementById("login-username");
  const pInput = document.getElementById("login-password");
  const username = uInput ? uInput.value.trim() : "";
  const password = pInput ? pInput.value : "";
  if (!username || !password) {
    alert("Harap isi username dan password sub-akun.");
    return;
  }
  loginAdmin(username, password);
}

function logoutAdmin() {
  AdminState.authToken = null;
  AdminState.activeAccount = null;
  window.authToken = null;
  window.activeAccount = null;
  localStorage.removeItem("program1_seller_jwt_token");
  localStorage.removeItem("program1_jwt_token");
  localStorage.removeItem("program1_seller_active_user");
  showSellerLoginScreen();
}

async function authFetch(url, options = {}) {
  const token = AdminState.authToken || window.authToken;
  if (!token) {
    logoutAdmin();
    throw new Error("Authentication required. Please log in.");
  }
  options.headers = options.headers || {};
  options.headers["Authorization"] = `Bearer ${token}`;
  const res = await fetch(url, options);
  if (res.status === 401) {
    console.warn("Session expired or unauthorized (401). Showing login screen...");
    logoutAdmin();
  }
  return res;
}

// --- NAVIGATION & RENDERING ---
function switchView(viewName, navEl) {
  const account = AdminState.activeAccount || window.activeAccount;
  if (account) {
    const isAdmin = account.role && account.role.toLowerCase().includes("admin");
    if (!isAdmin && account.accessible_menus && !account.accessible_menus.includes(viewName)) {
      alert(`⚠️ Akses Ditolak: Akun Anda (${account.role}) tidak memiliki izin untuk membuka menu '${viewName}'. Silakan hubungi Administrator.`);
      return;
    }
  }

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
    const titleEl = document.getElementById("view-title");
    const subEl = document.getElementById("view-subtitle");
    if (titleEl) titleEl.innerText = titleMap[viewName].title;
    if (subEl) subEl.innerText = titleMap[viewName].sub;
  }

  if (viewName === "settings") {
    if (typeof loadBreakGlassStatus === "function") loadBreakGlassStatus();
    if (typeof fetchUserAccounts === "function") fetchUserAccounts();
  }
  if (viewName === "customers") {
    if (typeof loadCustomersDirectory === "function") loadCustomersDirectory();
  }
}

function toggleSubmenu(id) {
  const el = document.getElementById(id);
  if (el) {
    el.style.display = el.style.display === "block" ? "none" : "block";
  }
}

async function loadData() {
  try {
    if (typeof fetchUserAccounts === "function") await fetchUserAccounts();
    const [analyticsRes, channelsRes, ordersRes, stocksRes, catalogRes] = await Promise.all([
      authFetch("/api/v1/analytics"),
      authFetch("/api/v1/channels"),
      authFetch("/api/v1/orders"),
      authFetch("/api/v1/inventory"),
      authFetch("/api/v1/catalog")
    ]);

    if (analyticsRes.ok) {
      const analytics = await analyticsRes.json();
      const revEl = document.getElementById("dash-revenue");
      const ordEl = document.getElementById("dash-orders");
      const prodEl = document.getElementById("dash-products");
      if (revEl) revEl.innerText = `Rp ${analytics.gross_revenue.toLocaleString("id-ID")}`;
      if (ordEl) ordEl.innerText = analytics.total_orders;
      if (prodEl) prodEl.innerText = analytics.active_products;
      if (typeof renderAnalyticsBreakdown === "function") {
        renderAnalyticsBreakdown(analytics.channel_breakdown);
      }
    }

    if (channelsRes.ok) {
      const channels = await channelsRes.json();
      if (typeof renderChannelsGrid === "function") {
        renderChannelsGrid(channels);
      }
    }

    if (ordersRes.ok) {
      const ordersData = await ordersRes.json();
      const orders = ordersData.data || ordersData;
      AdminState.currentOrders = orders;
      window.currentOrders = orders;
      if (typeof renderOrders === "function") {
        renderOrders(orders);
      }
    }

    if (stocksRes.ok) {
      const stocksData = await stocksRes.json();
      const stocks = stocksData.data || stocksData;
      AdminState.currentStocks = stocks;
      window.currentStocks = stocks;
      if (typeof renderGineeStockList === "function") {
        renderGineeStockList(stocks);
      }
    }

    if (catalogRes.ok) {
      const catalogData = await catalogRes.json();
      const catalog = catalogData.data || catalogData;
      if (typeof renderMasterProducts === "function") {
        renderMasterProducts(catalog);
      }
    }

    if (typeof loadLowStockAlerts === "function") {
      await loadLowStockAlerts();
    }
  } catch (e) {
    console.error("Error loading OMS dashboard data:", e);
  }
}

// Window Exports
window.escapeHtml = escapeHtml;
window.showAdminToast = showAdminToast;
window.showAdminConfirm = showAdminConfirm;
window.showAdminPrompt = showAdminPrompt;
window.showSellerLoginScreen = showSellerLoginScreen;
window.showAdminMainApp = showAdminMainApp;
window.loginAdmin = loginAdmin;
window.handleSellerLogin = handleSellerLogin;
window.logoutAdmin = logoutAdmin;
window.authFetch = authFetch;
window.switchView = switchView;
window.toggleSubmenu = toggleSubmenu;
window.loadData = loadData;

document.addEventListener("DOMContentLoaded", () => {
  // Authentication initialization: Check if authenticated
  const token = AdminState.authToken || window.authToken;
  if (!token) {
    showSellerLoginScreen();
  } else {
    showAdminMainApp();
    loadData();
  }
});
