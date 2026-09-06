/* ==========================================================================
   STOREFRONT TOAST & DIALOG ENGINE
   File: /crates/web/static/store/store-toast.js
   ========================================================================== */

function showToast(message, type = "info", duration = 4000) {
  let container = document.getElementById("store-toast-container");
  if (!container) {
    container = document.createElement("div");
    container.id = "store-toast-container";
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

// Override default window.alert so browser dialogs displaying host IP addresses never appear
window.alert = function(msg) {
  let t = "info";
  const str = String(msg);
  if (str.includes("🎉") || str.includes("✅") || str.includes("berhasil") || str.includes("Berhasil")) {
    t = "success";
  } else if (str.includes("Gagal") || str.includes("Error") || str.includes("error") || str.includes("❌")) {
    t = "error";
  } else if (str.includes("Harap") || str.includes("Silakan") || str.includes("🔒") || str.includes("⚠️") || str.includes("kosong")) {
    t = "warning";
  }
  showToast(str, t);
};

// --- CUSTOM IN-APP CONFIRMATION SYSTEM (REPLACES BROWSER CONFIRM POPUPS) ---
let confirmModalResolver = null;

function showConfirm(message, title = "Konfirmasi Tindakan", icon = "⚠️") {
  return new Promise((resolve) => {
    confirmModalResolver = resolve;
    const modal = document.getElementById("store-confirm-modal");
    const titleEl = document.getElementById("store-confirm-title");
    const msgEl = document.getElementById("store-confirm-message");
    const iconEl = document.getElementById("store-confirm-icon");
    if (titleEl) titleEl.textContent = title;
    if (msgEl) msgEl.textContent = message;
    if (iconEl) iconEl.textContent = icon;
    if (modal) {
      modal.style.zIndex = "400";
      modal.style.display = "flex";
    }
  });
}

function closeConfirmModal(result) {
  const modal = document.getElementById("store-confirm-modal");
  if (modal) modal.style.display = "none";
  if (typeof confirmModalResolver === "function") {
    confirmModalResolver(Boolean(result));
    confirmModalResolver = null;
  }
}

// Override native confirm & prompt as safety net to eliminate "192.168.6.xxx says:" browser popups
window.confirm = function(msg) {
  console.warn("Native window.confirm intercepted to prevent IP dialog:", msg);
  return true;
};
window.prompt = function(msg, defaultText) {
  console.warn("Native window.prompt intercepted to prevent IP dialog:", msg);
  return defaultText || "";
};

window.showToast = showToast;
window.showConfirm = showConfirm;
window.closeConfirmModal = closeConfirmModal;
