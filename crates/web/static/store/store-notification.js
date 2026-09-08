/* ==========================================================================
   STOREFRONT IN-APP NOTIFICATION MODULE
   File: /crates/web/static/store/store-notification.js
   ========================================================================== */

let isNotifOpen = false;
let notifPollInterval = null;

function getBuyerAuthToken() {
  return window.StoreState ? window.StoreState.buyerToken : window.buyerToken || localStorage.getItem("program1_buyer_token");
}

async function updateNotificationBadge() {
  const token = getBuyerAuthToken();
  const notifContainer = document.getElementById("buyer-notification-container");
  const badgeEl = document.getElementById("buyer-notif-badge");

  if (!notifContainer) return;

  if (!token) {
    notifContainer.style.display = "none";
    if (badgeEl) badgeEl.style.display = "none";
    if (notifPollInterval) {
      clearInterval(notifPollInterval);
      notifPollInterval = null;
    }
    return;
  }

  notifContainer.style.display = "inline-flex";

  try {
    const res = await fetch("/api/v1/buyer/notifications/count", {
      headers: { Authorization: `Bearer ${token}` }
    });
    if (res.ok) {
      const data = await res.json();
      const count = data.unread_count || 0;
      if (badgeEl) {
        if (count > 0) {
          badgeEl.textContent = count > 99 ? "99+" : count;
          badgeEl.style.display = "inline-flex";
        } else {
          badgeEl.style.display = "none";
        }
      }
    }
  } catch (err) {
    console.debug("Failed to fetch notification count:", err);
  }

  // Start poll if not already started
  if (!notifPollInterval) {
    notifPollInterval = setInterval(updateNotificationBadge, 30000);
  }
}

async function fetchStoreNotifications() {
  const token = getBuyerAuthToken();
  const listEl = document.getElementById("buyer-notif-list");
  if (!token || !listEl) return;

  listEl.innerHTML = '<div class="notif-empty">Memuat notifikasi...</div>';

  try {
    const res = await fetch("/api/v1/buyer/notifications?limit=25", {
      headers: { Authorization: `Bearer ${token}` }
    });
    if (!res.ok) {
      listEl.innerHTML = '<div class="notif-empty">Gagal memuat notifikasi.</div>';
      return;
    }
    const notifs = await res.json();
    renderStoreNotifications(notifs);
  } catch (err) {
    console.error("Error fetching notifications:", err);
    listEl.innerHTML = '<div class="notif-empty">Terjadi kesalahan jaringan.</div>';
  }
}

function renderStoreNotifications(notifs) {
  const listEl = document.getElementById("buyer-notif-list");
  if (!listEl) return;

  if (!notifs || notifs.length === 0) {
    listEl.innerHTML = '<div class="notif-empty">Belum ada notifikasi baru</div>';
    return;
  }

  listEl.innerHTML = "";
  notifs.forEach((n) => {
    const item = document.createElement("div");
    item.className = `notif-item ${!n.is_read ? "unread" : ""}`;
    item.onclick = () => handleStoreNotificationClick(n);

    let icon = "🔔";
    if (n.category === "order") icon = "📦";
    else if (n.category === "promo") icon = "🏷️";
    else if (n.category === "system") icon = "⚡";

    const timeAgo = formatStoreNotifTime(n.created_at);

    item.innerHTML = `
      <div class="notif-icon">${icon}</div>
      <div class="notif-content">
        <div class="notif-title-row">
          <span class="notif-title">${escapeHtml(n.title)}</span>
          ${!n.is_read ? '<span class="notif-dot"></span>' : ""}
        </div>
        <div class="notif-body">${escapeHtml(n.body)}</div>
        <div class="notif-time">${timeAgo}</div>
      </div>
    `;
    listEl.appendChild(item);
  });
}

function formatStoreNotifTime(dateStr) {
  if (!dateStr) return "";
  try {
    const date = new Date(dateStr);
    const now = new Date();
    const diffSec = Math.floor((now - date) / 1000);
    if (diffSec < 60) return "Baru saja";
    if (diffSec < 3600) return `${Math.floor(diffSec / 60)} m lalu`;
    if (diffSec < 86400) return `${Math.floor(diffSec / 3600)} jam lalu`;
    return `${Math.floor(diffSec / 86400)} hari lalu`;
  } catch (_) {
    return dateStr;
  }
}

function escapeHtml(str) {
  if (!str) return "";
  return str.replace(/[&<>'"]/g, 
    tag => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', "'": '&#39;', '"': '&quot;' }[tag] || tag)
  );
}

async function handleStoreNotificationClick(notif) {
  const token = getBuyerAuthToken();
  if (!notif.is_read && token) {
    try {
      await fetch(`/api/v1/buyer/notifications/${notif.id}/read`, {
        method: "POST",
        headers: { Authorization: `Bearer ${token}` }
      });
      updateNotificationBadge();
    } catch (err) {
      console.error("Failed to mark notification as read:", err);
    }
  }

  // Action based on category or link
  if (notif.link) {
    window.location.href = notif.link;
  } else if (notif.category === "order") {
    if (typeof switchStoreTab === "function") {
      switchStoreTab("orders");
    } else if (typeof openBuyerOrdersModal === "function") {
      openBuyerOrdersModal();
    }
  }
  toggleStoreNotifications(false);
}

async function markAllStoreNotificationsRead() {
  const token = getBuyerAuthToken();
  if (!token) return;

  try {
    const res = await fetch("/api/v1/buyer/notifications/read-all", {
      method: "POST",
      headers: { Authorization: `Bearer ${token}` }
    });
    if (res.ok) {
      updateNotificationBadge();
      fetchStoreNotifications();
      if (typeof showToast === "function") {
        showToast("Semua notifikasi ditandai telah dibaca", "success");
      }
    }
  } catch (err) {
    console.error("Failed to mark all as read:", err);
  }
}

function toggleStoreNotifications(forceState) {
  const dropdown = document.getElementById("buyer-notification-dropdown");
  if (!dropdown) return;

  if (typeof forceState === "boolean") {
    isNotifOpen = forceState;
  } else {
    isNotifOpen = !isNotifOpen;
  }

  if (isNotifOpen) {
    dropdown.style.display = "block";
    fetchStoreNotifications();
  } else {
    dropdown.style.display = "none";
  }
}

// Global click outside to close dropdown
document.addEventListener("click", (e) => {
  const container = document.getElementById("buyer-notification-container");
  if (container && !container.contains(e.target) && isNotifOpen) {
    toggleStoreNotifications(false);
  }
});

// Hook into DOM ready
document.addEventListener("DOMContentLoaded", () => {
  updateNotificationBadge();
});

// Expose globals
window.updateNotificationBadge = updateNotificationBadge;
window.toggleStoreNotifications = toggleStoreNotifications;
window.markAllStoreNotificationsRead = markAllStoreNotificationsRead;
