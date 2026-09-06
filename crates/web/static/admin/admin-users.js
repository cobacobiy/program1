/* ==========================================================================
   ADMIN USERS, RBAC & BREAK-GLASS MODULE
   File: /crates/web/static/admin/admin-users.js
   ========================================================================== */

async function fetchUserAccounts() {
  try {
    const res = await authFetch("/api/v1/users/accounts");
    if (res.ok) {
      const userAccounts = await res.json();
      if (window.AdminState) window.AdminState.userAccounts = userAccounts;
      window.userAccounts = userAccounts;

      const savedUser = localStorage.getItem("program1_seller_active_user");
      let activeAccount = window.AdminState ? window.AdminState.activeAccount : window.activeAccount;

      if (savedUser) {
        activeAccount = userAccounts.find(a => a.username === savedUser) || userAccounts[0];
      } else if (!activeAccount && userAccounts.length > 0) {
        activeAccount = userAccounts.find(a => a.username === "admin") || userAccounts[0];
      } else if (activeAccount) {
        activeAccount = userAccounts.find(a => a.id === activeAccount.id) || userAccounts[0];
      }

      if (window.AdminState) window.AdminState.activeAccount = activeAccount;
      window.activeAccount = activeAccount;

      renderUserAccountSwitcher();
      renderUserAccountsTable();
      applyRBACPermissions(activeAccount);
    }
  } catch (e) {
    console.error("Error fetching user accounts:", e);
  }
}

function getRoleBadge(role) {
  const r = (role || "").toLowerCase();
  if (r.includes("admin")) {
    return `<span class="badge-role" style="background:rgba(239,68,68,0.2); color:#fca5a5; font-size:0.65rem; font-weight:700">ADMIN</span>`;
  }
  if (r.includes("manager")) {
    return `<span class="badge-role" style="background:rgba(59,130,246,0.2); color:#93c5fd; font-size:0.65rem; font-weight:700">MANAGER</span>`;
  }
  if (r.includes("cs") || r.includes("support")) {
    return `<span class="badge-role" style="background:rgba(245,158,11,0.2); color:#fcd34d; font-size:0.65rem; font-weight:700">CS</span>`;
  }
  return `<span class="badge-role" style="background:rgba(16,185,129,0.2); color:#6ee7b7; font-size:0.65rem; font-weight:700">STAFF</span>`;
}

function renderUserAccountSwitcher() {
  const activeAccount = window.AdminState ? window.AdminState.activeAccount : window.activeAccount;
  const userAccounts = window.AdminState ? window.AdminState.userAccounts : (window.userAccounts || []);
  if (!activeAccount) return;

  const avatarEl = document.getElementById("header-user-avatar");
  const nameEl = document.getElementById("header-user-name");
  const roleEl = document.getElementById("header-user-role");
  if (avatarEl) avatarEl.innerText = activeAccount.full_name.charAt(0).toUpperCase();
  if (nameEl) nameEl.innerText = activeAccount.full_name;
  if (roleEl) roleEl.innerText = activeAccount.role;

  const dropdownList = document.getElementById("user-dropdown-list");
  if (!dropdownList) return;

  dropdownList.innerHTML = userAccounts.map(acc => {
    const isAdmin = acc.role.toLowerCase().includes("admin");
    const menuLabel = isAdmin ? "Semua 16 Menu" : `${acc.accessible_menus.length} Menu`;
    return `
    <div class="dropdown-item ${acc.id === activeAccount.id ? "active" : ""}" onclick="switchActiveAccount('${acc.id}')">
      <div>
        <div style="display:flex; align-items:center; gap:0.4rem">
          <span style="font-size:0.8rem; font-weight:600; color:#fff">${acc.full_name}</span>
          ${getRoleBadge(acc.role)}
        </div>
        <div style="font-size:0.7rem; color:var(--text-muted)">${acc.role} (@${acc.username})</div>
      </div>
      <span style="font-size:0.7rem; color:${isAdmin ? 'var(--purple)' : 'var(--cyan)'}; font-weight:700">${menuLabel}</span>
    </div>
  `}).join("");
}

function toggleUserDropdown() {
  const menu = document.getElementById("user-dropdown-menu");
  if (menu) menu.classList.toggle("show");
}

window.addEventListener("click", (e) => {
  if (!e.target.closest(".user-switcher-container")) {
    const dropdown = document.getElementById("user-dropdown-menu");
    if (dropdown) dropdown.classList.remove("show");
  }
});

async function switchActiveAccount(userId) {
  const userAccounts = window.AdminState ? window.AdminState.userAccounts : (window.userAccounts || []);
  const acc = userAccounts.find(a => a.id === userId);
  if (acc) {
    const success = await loginAdmin(acc.username, "admin123");
    if (success) {
      if (window.AdminState) window.AdminState.activeAccount = acc;
      window.activeAccount = acc;
      renderUserAccountSwitcher();
      applyRBACPermissions(acc);
      const dropdown = document.getElementById("user-dropdown-menu");
      if (dropdown) dropdown.classList.remove("show");
    }
  }
}

function applyRBACPermissions(account) {
  if (!account) return;
  const navItems = document.querySelectorAll(".nav-item[data-menu-id]");
  let activeTabVisible = false;
  const isAdmin = account.role && account.role.toLowerCase().includes("admin");

  navItems.forEach(item => {
    const menuId = item.getAttribute("data-menu-id");
    if (isAdmin || (account.accessible_menus && account.accessible_menus.includes(menuId))) {
      item.style.display = "flex";
      if (item.classList.contains("active")) {
        activeTabVisible = true;
      }
    } else {
      item.style.display = "none";
    }
  });

  if (!activeTabVisible) {
    if (isAdmin) {
      const firstNavItem = document.querySelector(`.nav-item[data-menu-id="dashboard"]`) || navItems[0];
      if (firstNavItem) switchView("dashboard", firstNavItem);
    } else if (account.accessible_menus && account.accessible_menus.length > 0) {
      const firstMenu = account.accessible_menus[0];
      const firstNavItem = document.querySelector(`.nav-item[data-menu-id="${firstMenu}"]`);
      if (firstNavItem) {
        switchView(firstMenu, firstNavItem);
      }
    }
  }
}

function renderUserAccountsTable() {
  const tbody = document.getElementById("user-accounts-tbody");
  if (!tbody) return;
  const userAccounts = window.AdminState ? window.AdminState.userAccounts : (window.userAccounts || []);

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
        <strong style="color:var(--cyan)">${u.accessible_menus.length} / ${window.ALL_MENU_ITEMS ? window.ALL_MENU_ITEMS.length : 16} Menu</strong>
        <div style="font-size:0.7rem; color:var(--text-muted); max-width:260px; text-overflow:ellipsis; overflow:hidden; white-space:nowrap">
          ${u.accessible_menus.join(", ")}
        </div>
      </td>
      <td>
        ${u.is_active
          ? '<span style="color:var(--emerald); font-weight:600">● Active</span>'
          : '<span style="color:var(--rose); font-weight:600">○ Nonaktif (Break-Glass)</span>'
        }
      </td>
      <td>
        <button class="btn-sm btn-action" onclick="openEditPermissionsModal('${u.id}')">⚙️ Edit Hak Akses</button>
      </td>
    </tr>
  `).join("");
}

function openEditPermissionsModal(userId) {
  const userAccounts = window.AdminState ? window.AdminState.userAccounts : (window.userAccounts || []);
  const user = userAccounts.find(u => u.id === userId);
  if (!user) return;
  document.getElementById("edit-perm-user-id").value = user.id;
  document.getElementById("edit-perm-user-label").value = `${user.full_name} (${user.role})`;

  const isAdmin = user.role && user.role.toLowerCase().includes("admin");
  const banner = document.getElementById("edit-perm-admin-banner");
  if (banner) {
    banner.style.display = isAdmin ? "block" : "none";
  }

  const items = window.ALL_MENU_ITEMS || [];
  const grid = document.getElementById("perm-checkboxes-grid");
  if (grid) {
    grid.innerHTML = items.map(m => {
      const isChecked = isAdmin || user.accessible_menus.includes(m.id);
      const isDisabled = isAdmin ? "disabled" : "";
      return `
      <label class="perm-item" style="${isAdmin ? 'opacity:0.75; cursor:not-allowed;' : ''}">
        <input type="checkbox" name="perm_menu" value="${m.id}" ${isChecked ? "checked" : ""} ${isDisabled}>
        <span>${m.label}</span>
      </label>
    `}).join("");
  }
  document.getElementById("edit-permissions-modal").style.display = "flex";
}

function closeEditPermissionsModal() {
  const modal = document.getElementById("edit-permissions-modal");
  if (modal) modal.style.display = "none";
}

function toggleAllPermCheckboxes(checked) {
  document.querySelectorAll('#perm-checkboxes-grid input[name="perm_menu"]:not([disabled])').forEach(cb => cb.checked = checked);
}

function onRolePresetChange(preset) {
  const roleInput = document.getElementById("create-role");
  if (preset === "custom") {
    roleInput.value = "";
    roleInput.focus();
    return;
  }
  roleInput.value = preset;

  const checkboxes = document.querySelectorAll('#create-perm-checkboxes-grid input[name="create_perm_menu"]');
  if (preset.toLowerCase().includes("admin")) {
    checkboxes.forEach(cb => { cb.checked = true; cb.disabled = true; });
  } else {
    checkboxes.forEach(cb => { cb.disabled = false; });
    let defaultMenus = [];
    if (preset === "Warehouse Manager") {
      defaultMenus = ["dashboard", "master_products", "channel_products", "stocks", "warehouses", "logistics"];
    } else if (preset === "Staff Gudang & Stok") {
      defaultMenus = ["dashboard", "stocks", "master_products"];
    } else if (preset === "Finance Officer") {
      defaultMenus = ["dashboard", "orders", "reports", "finances"];
    } else if (preset === "Customer Support") {
      defaultMenus = ["dashboard", "orders", "customers", "chat", "service"];
    } else {
      defaultMenus = ["dashboard", "orders"];
    }
    checkboxes.forEach(cb => {
      cb.checked = defaultMenus.includes(cb.value);
    });
  }
}

function openCreateUserModal() {
  document.getElementById("create-username").value = "";
  document.getElementById("create-fullname").value = "";
  const presetSelect = document.getElementById("create-role-preset");
  if (presetSelect) presetSelect.value = "Customer Support";
  document.getElementById("create-role").value = "Customer Support";

  const csMenus = ["dashboard", "orders", "customers", "chat", "service"];
  const grid = document.getElementById("create-perm-checkboxes-grid");
  const items = window.ALL_MENU_ITEMS || [];
  if (grid) {
    grid.innerHTML = items.map(m => `
      <label class="perm-item">
        <input type="checkbox" name="create_perm_menu" value="${m.id}" ${csMenus.includes(m.id) ? "checked" : ""}>
        <span>${m.label}</span>
      </label>
    `).join("");
  }
  document.getElementById("create-user-modal").style.display = "flex";
}

function closeCreateUserModal() {
  const modal = document.getElementById("create-user-modal");
  if (modal) modal.style.display = "none";
}

// --- BREAK-GLASS DEVELOPER ACCESS ---
async function loadBreakGlassStatus() {
  const statusBadge = document.getElementById("break-glass-status-badge");
  const activeBanner = document.getElementById("break-glass-active-banner");
  const infoText = document.getElementById("break-glass-info-text");
  const inactiveBox = document.getElementById("break-glass-inactive-form-box");
  if (!statusBadge) return;

  try {
    const res = await authFetch("/api/v1/admin/break-glass/status");
    if (res.ok) {
      const status = await res.json();
      if (status.is_active) {
        statusBadge.innerHTML = `<span class="badge-role" style="background:rgba(239,68,68,0.25); color:#fca5a5; font-size:0.8rem">🔴 AKTIF HINGGA ${new Date(status.expires_at).toLocaleTimeString('id-ID')}</span>`;
        if (activeBanner) activeBanner.style.display = "block";
        if (inactiveBox) inactiveBox.style.display = "none";
        if (infoText) {
          const exp = status.expires_at ? new Date(status.expires_at).toLocaleString('id-ID') : "-";
          infoText.innerHTML = `<strong>Alasan:</strong> ${status.reason || "-"} &nbsp;|&nbsp; <strong>Kedaluwarsa:</strong> ${exp} (${status.time_remaining_minutes} menit tersisa) &nbsp;|&nbsp; <strong>Diaktifkan Oleh:</strong> ${status.activated_by || "Admin"}`;
        }
      } else {
        statusBadge.innerHTML = `<span class="badge-role" style="background:rgba(16,185,129,0.15); color:#6ee7b7; font-size:0.8rem">🟢 NONAKTIF (AMAN)</span>`;
        if (activeBanner) activeBanner.style.display = "none";
        if (inactiveBox) inactiveBox.style.display = "block";
      }
    }
  } catch (e) {
    console.error("Error fetching break-glass status:", e);
  }
}

async function handleActivateBreakGlass(e) {
  if (e) e.preventDefault();
  const reasonInput = document.getElementById("bg-reason");
  const durationInput = document.getElementById("bg-duration");
  const reason = reasonInput ? reasonInput.value.trim() : "";
  const duration_hours = durationInput ? parseInt(durationInput.value, 10) : 2;

  if (!reason) {
    alert("Harap masukkan alasan aktivasi mode darurat.");
    return;
  }

  try {
    const res = await authFetch("/api/v1/admin/break-glass/activate", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ reason, duration_hours })
    });

    if (res.ok) {
      alert(`🚨 Akses Darurat Break-Glass BERHASIL DIAKTIFKAN!\nAkun dev_support aktif untuk ${duration_hours} jam.`);
      if (reasonInput) reasonInput.value = "";
      await loadBreakGlassStatus();
      await fetchUserAccounts();
    } else {
      const err = await res.json();
      alert(`Gagal mengaktifkan break-glass: ${err.message || err.error || JSON.stringify(err)}`);
    }
  } catch (e) {
    console.error("Activate break-glass error:", e);
    alert(`Error: ${e.message}`);
  }
}

async function handleDeactivateBreakGlass() {
  if (!await showAdminConfirm("Apakah Anda yakin ingin segera menonaktifkan akses darurat developer? Akun dev_support akan dinonaktifkan seketika.", "Nonaktifkan Akses Darurat", "🔒")) {
    return;
  }

  try {
    const res = await authFetch("/api/v1/admin/break-glass/deactivate", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ reason: "Manual deactivation by seller owner via dashboard" })
    });

    if (res.ok) {
      alert("✅ Akses Darurat Developer telah dinonaktifkan. Akun dev_support kembali dikunci.");
      await loadBreakGlassStatus();
      await fetchUserAccounts();
    } else {
      const err = await res.json();
      alert(`Gagal menonaktifkan break-glass: ${err.message || err.error || JSON.stringify(err)}`);
    }
  } catch (e) {
    console.error("Deactivate break-glass error:", e);
    alert(`Error: ${e.message}`);
  }
}

// Window Exports
window.fetchUserAccounts = fetchUserAccounts;
window.getRoleBadge = getRoleBadge;
window.renderUserAccountSwitcher = renderUserAccountSwitcher;
window.toggleUserDropdown = toggleUserDropdown;
window.switchActiveAccount = switchActiveAccount;
window.applyRBACPermissions = applyRBACPermissions;
window.renderUserAccountsTable = renderUserAccountsTable;
window.openEditPermissionsModal = openEditPermissionsModal;
window.closeEditPermissionsModal = closeEditPermissionsModal;
window.toggleAllPermCheckboxes = toggleAllPermCheckboxes;
window.onRolePresetChange = onRolePresetChange;
window.openCreateUserModal = openCreateUserModal;
window.closeCreateUserModal = closeCreateUserModal;
window.loadBreakGlassStatus = loadBreakGlassStatus;
window.handleActivateBreakGlass = handleActivateBreakGlass;
window.handleDeactivateBreakGlass = handleDeactivateBreakGlass;

document.addEventListener("DOMContentLoaded", () => {
  const editPermForm = document.getElementById("edit-permissions-form");
  if (editPermForm) {
    editPermForm.onsubmit = async (e) => {
      e.preventDefault();
      const userId = document.getElementById("edit-perm-user-id").value;
      const userAccounts = window.AdminState ? window.AdminState.userAccounts : (window.userAccounts || []);
      const user = userAccounts.find(u => u.id === userId);
      const isAdmin = user && user.role && user.role.toLowerCase().includes("admin");
      const items = window.ALL_MENU_ITEMS || [];
      const checked = isAdmin
        ? items.map(m => m.id)
        : Array.from(document.querySelectorAll('#perm-checkboxes-grid input[name="perm_menu"]:checked')).map(cb => cb.value);

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
      const isAdmin = role.toLowerCase().includes("admin");
      const items = window.ALL_MENU_ITEMS || [];
      const accessible_menus = isAdmin
        ? items.map(m => m.id)
        : Array.from(document.querySelectorAll('#create-perm-checkboxes-grid input[name="create_perm_menu"]:checked')).map(cb => cb.value);

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
});
