/* ==========================================================================
   STOREFRONT MAIN & SHARED STATE ENGINE
   File: /crates/web/static/store/store-main.js
   ========================================================================== */

const StoreState = {
  cart: [],
  catalog: [],
  activeCategory: "ALL",
  searchQuery: "",
  buyerToken: localStorage.getItem("program1_buyer_token") || null,
  activeBuyer: null,
  buyerAddresses: [],
  selectedAddressId: null,
  storeWhatsAppNumber: "085810007735",
  googleClientId: null,
  gisRenderAttempts: 0,
  storeInfo: null,
  otpTimerInterval: null,
  otpSecondsRemaining: 300,
  currentOtpPhone: "",
  inAppChatMessages: []
};

window.StoreState = StoreState;

// Provide backward-compatible top-level property bindings
["cart", "catalog", "activeCategory", "searchQuery", "buyerToken", "activeBuyer",
 "buyerAddresses", "selectedAddressId", "storeWhatsAppNumber", "googleClientId",
 "gisRenderAttempts", "storeInfo", "otpTimerInterval", "otpSecondsRemaining",
 "currentOtpPhone", "inAppChatMessages"].forEach(prop => {
  Object.defineProperty(window, prop, {
    get() { return StoreState[prop]; },
    set(val) { StoreState[prop] = val; },
    configurable: true
  });
});

// --- THEME ENGINE (DARK / LIGHT) ---
function initStoreTheme() {
  const savedTheme = localStorage.getItem("shopee_store_theme") || "dark";
  applyStoreTheme(savedTheme);
}

function applyStoreTheme(theme) {
  document.documentElement.setAttribute("data-theme", theme);
  localStorage.setItem("shopee_store_theme", theme);
  const icon = document.getElementById("store-theme-icon");
  if (icon) {
    icon.innerText = theme === "light" ? "🌙" : "☀️";
  }
}

function toggleStoreTheme() {
  const currentTheme = document.documentElement.getAttribute("data-theme") || "dark";
  const newTheme = currentTheme === "light" ? "dark" : "light";
  applyStoreTheme(newTheme);
}

// --- INITIALIZE STOREFRONT ---
async function initStore() {
  try {
    initStoreTheme();
    // 1. Check Storefront Buyer Session
    if (typeof checkBuyerSession === "function") {
      checkBuyerSession();
    }
    if (typeof fetchBuyerAuthConfig === "function") {
      await fetchBuyerAuthConfig();
    }

    // 2. Fetch Store Information
    const infoRes = await fetch('/api/v1/store/info');
    if (infoRes.ok) {
      StoreState.storeInfo = await infoRes.json();
      StoreState.storeWhatsAppNumber = StoreState.storeInfo.whatsapp_number || "085810007735";
      const nameEl = document.getElementById('header-store-name');
      if (nameEl) nameEl.innerText = StoreState.storeInfo.store_name || "AURA Storefront";

      const titleEl = document.getElementById('page-title');
      if (titleEl) titleEl.innerText = `${StoreState.storeInfo.store_name || "AURA Storefront"} — Shopee Official Store`;

      if (typeof updateFloatingChatWidget === "function") {
        updateFloatingChatWidget();
      }
    }

    // 3. Fetch Catalog Products
    const res = await fetch('/api/v1/catalog?page=1&page_size=100');
    if (res.ok) {
      const respData = await res.json();
      StoreState.catalog = respData.data || respData;
      if (typeof renderCategoryPills === "function") renderCategoryPills();
      if (typeof renderCatalog === "function") renderCatalog();
    }

    // 4. Start Flash Sale Countdown Timer
    if (typeof startFlashSaleTimer === "function") startFlashSaleTimer();

    // 5. Setup Search Event Listeners
    if (typeof setupSearchListener === "function") setupSearchListener();
  } catch (e) {
    console.error("Failed to initialize storefront data:", e);
  }
}

// --- STORE NAVIGATION TABS ENGINE ---
function switchStoreTab(tabId) {
  const validTabs = ['home', 'orders', 'addresses', 'profile'];
  if (!validTabs.includes(tabId)) tabId = 'home';
  StoreState.activeTab = tabId;

  // Check auth for protected buyer views
  if ((tabId === 'orders' || tabId === 'addresses' || tabId === 'profile') && !StoreState.buyerToken) {
    if (typeof showToast === "function") {
      showToast("Silakan masuk terlebih dahulu untuk mengakses menu ini.", "warning");
    }
    if (typeof openBuyerLoginModal === "function") {
      openBuyerLoginModal();
    }
    return;
  }

  // Update top tabs
  validTabs.forEach(t => {
    const btn = document.getElementById(`tab-btn-${t}`);
    const view = document.getElementById(`view-store-${t}`);
    if (btn) {
      if (t === tabId) btn.classList.add('active');
      else btn.classList.remove('active');
    }
    if (view) {
      if (t === tabId) {
        view.classList.add('active');
        view.style.display = 'block';
      } else {
        view.classList.remove('active');
        view.style.display = 'none';
      }
    }
  });

  // Update mobile bottom nav
  const mobMap = { home: 'mob-nav-home', orders: 'mob-nav-orders', profile: 'mob-nav-profile' };
  Object.keys(mobMap).forEach(key => {
    const el = document.getElementById(mobMap[key]);
    if (el) {
      if (key === tabId) el.classList.add('active');
      else el.classList.remove('active');
    }
  });

  window.scrollTo({ top: 0, behavior: 'smooth' });

  // Trigger tab-specific loaders
  if (tabId === 'orders' && typeof fetchBuyerOrdersDashboard === "function") {
    fetchBuyerOrdersDashboard();
  } else if (tabId === 'addresses' && typeof fetchBuyerAddressesDashboard === "function") {
    fetchBuyerAddressesDashboard();
  } else if (tabId === 'profile' && typeof renderBuyerProfileDashboard === "function") {
    renderBuyerProfileDashboard();
  }
}

window.initStore = initStore;
window.initStoreTheme = initStoreTheme;
window.applyStoreTheme = applyStoreTheme;
window.toggleStoreTheme = toggleStoreTheme;
window.switchStoreTab = switchStoreTab;

document.addEventListener("DOMContentLoaded", () => {
  initStore();
});

