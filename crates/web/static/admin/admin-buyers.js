/* ==========================================================================
   ADMIN BUYERS DIRECTORY & CRM MODULE
   File: /crates/web/static/admin/admin-buyers.js
   ========================================================================== */

async function loadCustomersDirectory() {
  const tbody = document.getElementById("cust-table-tbody");
  if (tbody) {
    tbody.innerHTML = `<tr><td colspan="7" style="text-align:center; padding:2rem; color:var(--text-muted)">Memuat data pelanggan...</td></tr>`;
  }

  try {
    const res = await authFetch("/api/v1/admin/buyers");
    if (!res.ok) {
      if (tbody) {
        tbody.innerHTML = `<tr><td colspan="7" style="text-align:center; padding:2rem; color:var(--rose)">Gagal memuat data pelanggan (${res.status}).</td></tr>`;
      }
      return;
    }

    const buyersData = await res.json();
    const buyers = buyersData.data || buyersData;
    if (window.AdminState) window.AdminState.currentCustomers = buyers;
    window.currentCustomers = buyers;

    // Update stat cards
    const totalCount = buyers.length;
    const verifiedCount = buyers.filter(c => c.phone_verified).length;
    const activeCount = buyers.filter(c => c.is_active).length;

    const statTotalEl = document.getElementById("cust-stat-total");
    if (statTotalEl) statTotalEl.innerText = totalCount.toString();

    const statVerifiedEl = document.getElementById("cust-stat-verified");
    if (statVerifiedEl) statVerifiedEl.innerText = verifiedCount.toString();

    const statActiveEl = document.getElementById("cust-stat-active");
    if (statActiveEl) statActiveEl.innerText = activeCount.toString();

    filterCustomersList();

    const currentCustomerTab = window.AdminState ? window.AdminState.currentCustomerTab : (window.currentCustomerTab || "list");
    if (currentCustomerTab === "activity") {
      loadBuyerActivityLogs();
    }
  } catch (err) {
    console.error("Error loading buyers directory:", err);
    if (tbody) {
      tbody.innerHTML = `<tr><td colspan="7" style="text-align:center; padding:2rem; color:var(--rose)">Error: ${err.message}</td></tr>`;
    }
  }
}

function filterCustomersList() {
  const searchInput = document.getElementById("cust-search-input");
  const query = (searchInput ? searchInput.value : "").trim().toLowerCase();
  const filterStatus = document.getElementById("cust-filter-status") ? document.getElementById("cust-filter-status").value : "all";
  const currentCustomers = window.AdminState ? window.AdminState.currentCustomers : (window.currentCustomers || []);

  let filtered = currentCustomers.filter(c => {
    const matchQuery = !query ||
      (c.full_name && c.full_name.toLowerCase().includes(query)) ||
      (c.email && c.email.toLowerCase().includes(query)) ||
      (c.phone_number && c.phone_number.toLowerCase().includes(query));

    let matchStatus = true;
    if (filterStatus === "active") matchStatus = c.is_active === true;
    if (filterStatus === "inactive") matchStatus = c.is_active === false;

    return matchQuery && matchStatus;
  });

  renderCustomersTable(filtered);
}

function renderCustomersTable(buyers) {
  const tbody = document.getElementById("cust-table-tbody");
  if (!tbody) return;

  if (buyers.length === 0) {
    tbody.innerHTML = `<tr><td colspan="7" style="text-align:center; padding:2rem; color:var(--text-muted)">Tidak ada akun pelanggan yang cocok.</td></tr>`;
    return;
  }

  tbody.innerHTML = buyers.map(b => {
    const initial = (b.full_name || b.email || "B").charAt(0).toUpperCase();
    const isGoogle = !!b.google_sub;
    const methodBadge = isGoogle
      ? `<span style="background:rgba(59,130,246,0.15); color:var(--blue); padding:0.2rem 0.5rem; border-radius:6px; font-size:0.75rem; font-weight:600;">🌐 Google</span>`
      : `<span style="background:rgba(139,92,246,0.15); color:var(--purple); padding:0.2rem 0.5rem; border-radius:6px; font-size:0.75rem; font-weight:600;">✉️ Email/Pass</span>`;

    const phoneBadge = b.phone_verified
      ? `<span style="background:rgba(16,185,129,0.15); color:var(--emerald); padding:0.15rem 0.4rem; border-radius:4px; font-size:0.7rem; font-weight:600;">✔ Verified</span>`
      : `<span style="background:rgba(245,158,11,0.15); color:var(--amber); padding:0.15rem 0.4rem; border-radius:4px; font-size:0.7rem;">⏳ Unverified</span>`;

    const statusBadge = b.is_active
      ? `<span style="background:rgba(16,185,129,0.15); color:var(--emerald); padding:0.2rem 0.55rem; border-radius:6px; font-size:0.75rem; font-weight:600;">Aktif</span>`
      : `<span style="background:rgba(239,68,68,0.15); color:var(--rose); padding:0.2rem 0.55rem; border-radius:6px; font-size:0.75rem; font-weight:600;">Nonaktif</span>`;

    const createdDate = b.created_at ? new Date(b.created_at).toLocaleDateString("id-ID", { year: "numeric", month: "short", day: "numeric" }) : "-";

    const actionBtn = b.is_active
      ? `<button class="btn-sm" style="background:rgba(239,68,68,0.15); color:#fca5a5; border:1px solid rgba(239,68,68,0.3); font-size:0.75rem; padding:0.25rem 0.6rem; cursor:pointer;" onclick="toggleBuyerStatus('${b.id}', false)">Nonaktifkan</button>`
      : `<button class="btn-sm" style="background:rgba(16,185,129,0.15); color:#6ee7b7; border:1px solid rgba(16,185,129,0.3); font-size:0.75rem; padding:0.25rem 0.6rem; cursor:pointer;" onclick="toggleBuyerStatus('${b.id}', true)">Aktifkan</button>`;

    return `
      <tr>
        <td>
          <div style="display:flex; align-items:center; gap:0.6rem;">
            <div style="width:32px; height:32px; border-radius:50%; background:linear-gradient(135deg, var(--cyan), var(--blue)); display:flex; align-items:center; justify-content:center; color:#fff; font-weight:700; font-size:0.85rem;">
              ${initial}
            </div>
            <div>
              <div style="font-weight:600; color:var(--text-main); font-size:0.85rem;">${escapeHtml(b.full_name || "-")}</div>
              <div style="font-size:0.7rem; color:var(--text-muted); font-family:'JetBrains Mono';">${b.id.substring(0, 8)}...</div>
            </div>
          </div>
        </td>
        <td style="color:var(--text-main); font-size:0.85rem;">${escapeHtml(b.email)}</td>
        <td>
          <div style="display:flex; align-items:center; gap:0.4rem;">
            <span style="font-size:0.85rem;">${escapeHtml(b.phone_number || "-")}</span>
            ${b.phone_number ? phoneBadge : ""}
          </div>
        </td>
        <td>${methodBadge}</td>
        <td style="font-size:0.8rem; color:var(--text-muted);">${createdDate}</td>
        <td>${statusBadge}</td>
        <td style="text-align:center;">${actionBtn}</td>
      </tr>
    `;
  }).join("");
}

async function toggleBuyerStatus(buyerId, newActiveStatus) {
  const actionText = newActiveStatus ? "mengaktifkan" : "menonaktifkan";
  if (!await showAdminConfirm(`Apakah Anda yakin ingin ${actionText} akun pelanggan ini?`, "Konfirmasi Status Pelanggan", "👥")) {
    return;
  }

  try {
    const res = await authFetch(`/api/v1/admin/buyers/${buyerId}/status`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ is_active: newActiveStatus })
    });

    if (res.ok) {
      alert(`✅ Status akun pelanggan berhasil diubah menjadi ${newActiveStatus ? "Aktif" : "Nonaktif"}.`);
      await loadCustomersDirectory();
    } else {
      const err = await res.json();
      alert(`❌ Gagal mengubah status: ${err.message || err.error || JSON.stringify(err)}`);
    }
  } catch (err) {
    console.error("Toggle buyer status error:", err);
    alert(`❌ Error: ${err.message}`);
  }
}

function switchCustomerTab(tabName, btnEl) {
  if (window.AdminState) window.AdminState.currentCustomerTab = tabName;
  window.currentCustomerTab = tabName;

  document.querySelectorAll("#view-customers .tabs-bar .tab-btn").forEach(btn => btn.classList.remove("active"));
  if (btnEl) btnEl.classList.add("active");

  const listSub = document.getElementById("cust-subview-list");
  const actSub = document.getElementById("cust-subview-activity");

  if (tabName === "list") {
    if (listSub) listSub.style.display = "block";
    if (actSub) actSub.style.display = "none";
  } else if (tabName === "activity") {
    if (listSub) listSub.style.display = "none";
    if (actSub) actSub.style.display = "block";
    loadBuyerActivityLogs();
  }
}

async function loadBuyerActivityLogs() {
  const tbody = document.getElementById("cust-activity-tbody");
  if (!tbody) return;

  tbody.innerHTML = `<tr><td colspan="4" style="text-align:center; padding:2rem; color:var(--text-muted)">Memuat riwayat aktivitas pembeli...</td></tr>`;

  try {
    const res = await authFetch("/api/v1/admin/buyers/activity");
    if (!res.ok) {
      tbody.innerHTML = `<tr><td colspan="4" style="text-align:center; padding:2rem; color:var(--rose)">Gagal memuat log aktivitas (${res.status}).</td></tr>`;
      return;
    }

    const logs = await res.json();
    if (logs.length === 0) {
      tbody.innerHTML = `<tr><td colspan="4" style="text-align:center; padding:2rem; color:var(--text-muted)">Belum ada log aktivitas pelanggan yang tercatat.</td></tr>`;
      return;
    }

    tbody.innerHTML = logs.map(l => {
      const timeStr = l.timestamp ? new Date(l.timestamp).toLocaleString("id-ID") : "-";
      const actorStr = l.actor_email || (l.actor_id ? l.actor_id.substring(0, 8) + "..." : "Sistem");

      let badgeBg = "rgba(59,130,246,0.15)";
      let badgeCol = "var(--blue)";
      if (l.action.includes("REGISTER")) {
        badgeBg = "rgba(16,185,129,0.15)";
        badgeCol = "var(--emerald)";
      } else if (l.action.includes("LOGIN")) {
        badgeBg = "rgba(139,92,246,0.15)";
        badgeCol = "var(--purple)";
      } else if (l.action.includes("STATUS")) {
        badgeBg = "rgba(245,158,11,0.15)";
        badgeCol = "var(--amber)";
      }

      let detailsStr = "-";
      if (typeof l.details === "string") {
        detailsStr = l.details;
      } else if (l.details) {
        detailsStr = JSON.stringify(l.details);
      }

      return `
        <tr>
          <td style="font-size:0.8rem; color:var(--text-muted); white-space:nowrap;">${timeStr}</td>
          <td style="font-weight:600; font-size:0.85rem; color:var(--text-main);">${escapeHtml(actorStr)}</td>
          <td>
            <span style="background:${badgeBg}; color:${badgeCol}; padding:0.2rem 0.5rem; border-radius:6px; font-size:0.75rem; font-weight:600; font-family:'JetBrains Mono';">
              ${escapeHtml(l.action)}
            </span>
          </td>
          <td style="font-size:0.8rem; color:var(--text-muted); max-width:320px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">${escapeHtml(detailsStr)}</td>
        </tr>
      `;
    }).join("");
  } catch (err) {
    console.error("Error loading buyer activity logs:", err);
    tbody.innerHTML = `<tr><td colspan="4" style="text-align:center; padding:2rem; color:var(--rose)">Error: ${err.message}</td></tr>`;
  }
}

// Window Exports
window.loadCustomersDirectory = loadCustomersDirectory;
window.filterCustomersList = filterCustomersList;
window.renderCustomersTable = renderCustomersTable;
window.toggleBuyerStatus = toggleBuyerStatus;
window.switchCustomerTab = switchCustomerTab;
window.loadBuyerActivityLogs = loadBuyerActivityLogs;
