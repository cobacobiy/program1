/* ==========================================================================
   STOREFRONT ADDRESS, CHECKOUT & ORDER FLOW ENGINE
   File: /crates/web/static/store/store-checkout.js
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

// --- BUYER ADDRESS BOOK ENGINE ---
async function fetchBuyerAddresses() {
  const token = window.StoreState ? window.StoreState.buyerToken : window.buyerToken;
  if (!token) return;
  try {
    const res = await buyerAuthFetch("/api/v1/buyer/addresses");
    if (res.ok) {
      const addrs = await res.json();
      if (window.StoreState) window.StoreState.buyerAddresses = addrs;
      window.buyerAddresses = addrs;

      const defaultAddr = addrs.find(a => a.is_default) || addrs[0];
      if (defaultAddr) {
        if (window.StoreState) window.StoreState.selectedAddressId = defaultAddr.id;
        window.selectedAddressId = defaultAddr.id;
      }
      renderBuyerAddressesList();
      renderConfirmedAddressInCheckout();
    }
  } catch (e) {
    console.error("Fetch buyer addresses error:", e);
  }
}

function renderBuyerAddressesList() {
  const container = document.getElementById("buyer-addresses-list");
  if (!container) return;

  const buyerAddresses = window.StoreState ? window.StoreState.buyerAddresses : (window.buyerAddresses || []);

  if (buyerAddresses.length === 0) {
    container.innerHTML = `<p style="font-size:0.8rem; color:var(--text-muted); text-align:center; padding:1rem">Belum ada alamat tersimpan.</p>`;
    return;
  }

  container.innerHTML = buyerAddresses.map(a => `
    <div class="address-item-card ${a.is_default ? 'is-default' : ''}">
      <div style="display:flex; justify-content:space-between; align-items:center">
        <div style="display:flex; align-items:center; gap:0.4rem">
          <span class="address-badge-label">${a.label || 'Rumah'}</span>
          ${a.is_default ? '<span class="address-badge-default">UTAMA / DEFAULT</span>' : ''}
        </div>
        <div style="display:flex; gap:0.4rem">
          ${!a.is_default ? `<button type="button" class="btn-sm-action" onclick="handleSetDefaultAddress('${a.id}')">Jadikan Utama</button>` : ''}
          <button type="button" class="btn-sm-action" style="color:var(--rose); border-color:rgba(239,68,68,0.3)" onclick="handleDeleteAddress('${a.id}')">Hapus</button>
        </div>
      </div>
      <div style="font-size:0.85rem; font-weight:700; color:var(--text-heading); margin-top:0.2rem">
        ${a.recipient_name} <span style="font-weight:400; font-size:0.8rem; color:var(--text-muted)">(${a.phone_number})</span>
      </div>
      <div style="font-size:0.8rem; color:var(--text-muted)">
        ${a.street_address}, Kec. ${a.subdistrict}, ${a.city}, ${a.province} ${a.postal_code}
      </div>
    </div>
  `).join('');

  renderDashboardAddressesList();
}

function renderDashboardAddressesList() {
  const container = document.getElementById("dashboard-addresses-list");
  if (!container) return;

  const buyerAddresses = window.StoreState ? window.StoreState.buyerAddresses : (window.buyerAddresses || []);

  if (buyerAddresses.length === 0) {
    container.innerHTML = `
      <div style="grid-column:1/-1; text-align:center; padding:3rem 1rem; color:var(--text-muted)">
        <div style="font-size:2.5rem; margin-bottom:0.5rem">📍</div>
        <p style="font-weight:600; color:var(--text-heading); margin-bottom:0.3rem">Belum Ada Alamat Pengiriman</p>
        <p style="font-size:0.85rem; margin-bottom:1rem">Tambahkan alamat utama untuk memudahkan proses pesanan dan pengiriman barang Anda.</p>
        <button type="button" class="btn-checkout" style="width:auto; padding:0.6rem 1.5rem" onclick="openAddAddressModal()">+ Tambah Alamat Pertama</button>
      </div>
    `;
    return;
  }

  container.innerHTML = buyerAddresses.map(a => `
    <div class="dashboard-address-card ${a.is_default ? 'is-default' : ''}">
      <div style="display:flex; justify-content:space-between; align-items:center">
        <div style="display:flex; align-items:center; gap:0.5rem">
          <span class="address-badge-label" style="font-weight:700">${escapeHtml(a.label || 'Rumah')}</span>
          ${a.is_default ? '<span class="address-default-badge">⭐ UTAMA / DEFAULT</span>' : ''}
        </div>
        <div style="display:flex; gap:0.4rem">
          ${!a.is_default ? `<button type="button" class="btn-sm-action" onclick="handleSetDefaultAddress('${a.id}')">Jadikan Utama</button>` : ''}
          <button type="button" class="btn-sm-action" style="color:var(--rose); border-color:rgba(239,68,68,0.3)" onclick="handleDeleteAddress('${a.id}')">Hapus</button>
        </div>
      </div>
      <div style="font-size:0.95rem; font-weight:700; color:var(--text-heading); margin-top:0.25rem">
        ${escapeHtml(a.recipient_name)} <span style="font-weight:400; font-size:0.85rem; color:var(--text-muted)">(${escapeHtml(a.phone_number)})</span>
      </div>
      <div style="font-size:0.85rem; color:var(--text); line-height:1.4">
        ${escapeHtml(a.street_address)}
      </div>
      <div style="font-size:0.8rem; color:var(--text-muted)">
        Kec. ${escapeHtml(a.subdistrict)}, ${escapeHtml(a.city)}, ${escapeHtml(a.province)} ${escapeHtml(a.postal_code)}
      </div>
    </div>
  `).join('');
}

async function fetchBuyerAddressesDashboard() {
  await fetchBuyerAddresses();
  renderDashboardAddressesList();
}

function openAddAddressModal() {
  const modal = document.getElementById("buyer-address-modal");
  const form = document.getElementById("buyer-address-form");
  if (form) form.reset();
  const idInput = document.getElementById("addr-id");
  if (idInput) idInput.value = "";

  const activeBuyer = window.StoreState ? window.StoreState.activeBuyer : window.activeBuyer;
  const buyerAddresses = window.StoreState ? window.StoreState.buyerAddresses : (window.buyerAddresses || []);

  if (activeBuyer && activeBuyer.full_name) {
    const recInput = document.getElementById("addr-recipient-name");
    if (recInput) recInput.value = activeBuyer.full_name;
  }
  if (activeBuyer && activeBuyer.phone_number) {
    const phoneInput = document.getElementById("addr-phone");
    if (phoneInput) phoneInput.value = activeBuyer.phone_number;
  }
  const defCheck = document.getElementById("addr-is-default");
  if (defCheck) defCheck.checked = buyerAddresses.length === 0;
  if (modal) {
    modal.style.zIndex = "300";
    modal.style.display = "flex";
  }
}

function closeBuyerAddressModal() {
  const modal = document.getElementById("buyer-address-modal");
  if (modal) modal.style.display = "none";
}

async function handleSaveBuyerAddress(e) {
  if (e) e.preventDefault();
  const id = document.getElementById("addr-id").value;
  const label = document.getElementById("addr-label").value.trim() || "Rumah";
  const recipient_name = document.getElementById("addr-recipient-name").value.trim();
  const phone_number = document.getElementById("addr-phone").value.trim();
  const street_address = document.getElementById("addr-street").value.trim();
  const subdistrict = document.getElementById("addr-subdistrict").value.trim();
  const city = document.getElementById("addr-city").value.trim();
  const province = document.getElementById("addr-province").value.trim();
  const postal_code = document.getElementById("addr-postal-code").value.trim();
  const is_default = document.getElementById("addr-is-default").checked;

  const payload = {
    label: label ? label : undefined,
    recipient_name,
    phone_number,
    street_address,
    subdistrict,
    city,
    province,
    postal_code,
    set_as_default: is_default
  };

  try {
    const url = id ? `/api/v1/buyer/addresses/${id}` : "/api/v1/buyer/addresses";
    const method = id ? "PUT" : "POST";
    const res = await buyerAuthFetch(url, {
      method,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload)
    });

    if (res.ok) {
      closeBuyerAddressModal();
      await fetchBuyerAddresses();
      if (typeof updateCartUI === "function") updateCartUI();
      showToast("Alamat pengiriman berhasil disimpan!", "success");
    } else {
      const err = await res.json();
      showToast(`Gagal menyimpan alamat: ${err.message || err.error || JSON.stringify(err)}`, "error");
    }
  } catch (e) {
    console.error("Save address error:", e);
    showToast(`Error: ${e.message}`, "error");
  }
}

async function handleSetDefaultAddress(id) {
  try {
    const res = await buyerAuthFetch(`/api/v1/buyer/addresses/${id}/default`, { method: "POST" });
    if (res.ok) {
      if (window.StoreState) window.StoreState.selectedAddressId = id;
      window.selectedAddressId = id;
      await fetchBuyerAddresses();
      if (typeof updateCartUI === "function") updateCartUI();
      showToast("Alamat utama berhasil diperbarui", "success");
    } else {
      const err = await res.json();
      showToast(`Gagal mengatur alamat default: ${err.message || err.error}`, "error");
    }
  } catch (e) {
    console.error("Set default address error:", e);
  }
}

async function handleDeleteAddress(id) {
  const confirmed = await showConfirm("Hapus alamat ini dari buku alamat Anda?", "Hapus Alamat", "🗑️");
  if (!confirmed) return;
  try {
    const res = await buyerAuthFetch(`/api/v1/buyer/addresses/${id}`, { method: "DELETE" });
    if (res.ok) {
      showToast("Alamat berhasil dihapus", "success");
      await fetchBuyerAddresses();
      if (typeof updateCartUI === "function") updateCartUI();
    } else {
      const err = await res.json();
      showToast(`Gagal menghapus alamat: ${err.message || err.error}`, "error");
    }
  } catch (e) {
    console.error("Delete address error:", e);
    showToast(`Error: ${e.message}`, "error");
  }
}

function renderConfirmedAddressInCheckout() {
  const card = document.getElementById("confirmed-address-card");
  if (!card) return;

  const buyerAddresses = window.StoreState ? window.StoreState.buyerAddresses : (window.buyerAddresses || []);
  let selectedAddressId = window.StoreState ? window.StoreState.selectedAddressId : window.selectedAddressId;

  if (buyerAddresses.length === 0) {
    card.innerHTML = `
      <div style="text-align:center; padding:0.5rem">
        <p style="font-size:0.85rem; color:var(--text-muted); margin-bottom:0.75rem">Belum ada alamat tersimpan.</p>
        <button type="button" class="btn-checkout" style="font-size:0.85rem; padding:0.4rem 1rem" onclick="openAddAddressModal()">+ Tambah Alamat Pengiriman</button>
      </div>
    `;
    return;
  }

  const currentAddr = buyerAddresses.find(a => a.id === selectedAddressId) || buyerAddresses[0];
  selectedAddressId = currentAddr.id;
  if (window.StoreState) window.StoreState.selectedAddressId = selectedAddressId;
  window.selectedAddressId = selectedAddressId;

  card.innerHTML = `
    <div>
      <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.5rem">
        <span class="addr-title">👤 ${currentAddr.recipient_name} <span class="address-badge-label">${currentAddr.label || 'Rumah'}</span></span>
        <select id="addr-switch-select" class="form-control" style="width:auto; font-size:0.75rem; padding:0.2rem 0.5rem" onchange="switchCheckoutAddress(this.value)">
          ${buyerAddresses.map(a => `<option value="${a.id}" ${a.id === selectedAddressId ? 'selected' : ''}>${a.label || 'Alamat'}: ${a.recipient_name} (${a.city})</option>`).join('')}
        </select>
      </div>
      <div class="addr-phone">📞 ${currentAddr.phone_number} (Terverifikasi OTP 🟢)</div>
      <div class="addr-text">
        📍 ${currentAddr.street_address}, Kec. ${currentAddr.subdistrict}, ${currentAddr.city}, ${currentAddr.province}
      </div>
      <div style="font-size:0.8rem; color:var(--cyan); font-family:'JetBrains Mono'; margin-top:0.25rem">
        Kode Pos: ${currentAddr.postal_code}
      </div>
    </div>
  `;
}

function switchCheckoutAddress(addrId) {
  if (window.StoreState) window.StoreState.selectedAddressId = addrId;
  window.selectedAddressId = addrId;
  renderConfirmedAddressInCheckout();
}

async function handleProcessCheckout(e) {
  if (e) e.preventDefault();

  const cart = window.StoreState ? window.StoreState.cart : (window.cart || []);
  const buyerToken = window.StoreState ? window.StoreState.buyerToken : window.buyerToken;
  const activeBuyer = window.StoreState ? window.StoreState.activeBuyer : window.activeBuyer;
  const buyerAddresses = window.StoreState ? window.StoreState.buyerAddresses : (window.buyerAddresses || []);
  const selectedAddressId = window.StoreState ? window.StoreState.selectedAddressId : window.selectedAddressId;

  if (cart.length === 0) {
    showToast("Keranjang belanja kosong.", "warning");
    return;
  }

  if (!buyerToken || !activeBuyer) {
    openBuyerLoginModal();
    return;
  }

  if (!activeBuyer.phone_verified) {
    openOtpModal();
    return;
  }

  const selectedAddr = buyerAddresses.find(a => a.id === selectedAddressId);
  if (!selectedAddr) {
    showToast("Harap tambahkan alamat pengiriman terlebih dahulu.", "warning");
    openAddAddressModal();
    return;
  }

  const items = cart.map(i => ({ product_id: i.product_id, quantity: i.quantity }));

  const payload = {
    address_id: selectedAddressId,
    items: items
  };

  const btnSubmit = document.getElementById("btn-submit-order");
  if (btnSubmit) {
    btnSubmit.disabled = true;
    btnSubmit.innerText = "Memproses Pesanan...";
  }

  try {
    const res = await buyerAuthFetch("/api/v1/orders", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload)
    });

    if (res.ok) {
      const orderData = await res.json();
      if (window.StoreState) window.StoreState.cart = [];
      window.cart = [];
      if (typeof updateCartUI === "function") updateCartUI();
      if (typeof closeCart === "function") closeCart();
      showToast(`🎉 Pesanan berhasil dibuat! Menyiapkan gerbang pembayaran...`, 'success', 4000);
      await initiateMidtransPayment(orderData.id);
    } else {
      const err = await res.json();
      showToast(`Checkout Gagal: ${err.message || err.error || JSON.stringify(err)}`, 'error');
    }
  } catch (err) {
    console.error("Checkout order error:", err);
    showToast(`Error saat membuat order: ${err.message}`, 'error');
  } finally {
    if (btnSubmit) {
      btnSubmit.disabled = false;
      btnSubmit.innerText = "🚀 Konfirmasi & Pesan Sekarang";
    }
  }
}

async function initiateMidtransPayment(orderId) {
  try {
    const res = await buyerAuthFetch("/api/v1/payments", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ order_id: orderId })
    });

    if (!res.ok) {
      const err = await res.json();
      showToast(`Gagal memuat pembayaran: ${err.message || err.error || 'Terjadi kesalahan'}`, 'error');
      return;
    }

    const payment = await res.json();

    if (window.snap && typeof window.snap.pay === "function" && payment.snap_token) {
      window.snap.pay(payment.snap_token, {
        onSuccess: function(result) {
          showToast('🎉 Pembayaran Berhasil Dikonfirmasi!', 'success');
          console.log('Payment success:', result);
        },
        onPending: function(result) {
          showToast('⏳ Menunggu Pembayaran. Silakan selesaikan transaksi Anda.', 'warning');
          console.log('Payment pending:', result);
        },
        onError: function(result) {
          showToast('❌ Pembayaran Gagal.', 'error');
          console.error('Payment error:', result);
        },
        onClose: function() {
          showToast('Jendela pembayaran ditutup. Anda dapat membayar nanti.', 'info');
        }
      });
    } else if (payment.snap_redirect_url) {
      showToast('Membuka halaman pembayaran Midtrans Snap...', 'info');
      window.open(payment.snap_redirect_url, '_blank');
    } else {
      showToast("Pesanan disimpan. Menunggu konfirmasi pembayaran.", "info");
    }
  } catch (err) {
    console.error("Initiate payment error:", err);
    showToast(`Error pembayaran: ${err.message}`, 'error');
  }
}

// --- BUYER ORDERS HISTORY & WORKFLOW ---
function openBuyerOrdersModal() {
  const token = window.StoreState ? window.StoreState.buyerToken : window.buyerToken;
  if (!token) {
    showToast("Silakan masuk terlebih dahulu untuk melihat riwayat pesanan Anda.", "warning");
    openBuyerLoginModal();
    return;
  }
  const modal = document.getElementById("buyer-orders-modal");
  if (modal) modal.style.display = "flex";
  fetchBuyerOrders();
}

function closeBuyerOrdersModal() {
  const modal = document.getElementById("buyer-orders-modal");
  if (modal) modal.style.display = "none";
}

function getBuyerOrderStatusBadge(status) {
  const s = (status || "").toLowerCase();
  switch (s) {
    case "pending":
      return `<span class="buyer-order-status-badge buyer-status-pending">⏳ Menunggu Pembayaran</span>`;
    case "paid":
      return `<span class="buyer-order-status-badge buyer-status-paid">💳 Pembayaran Berhasil</span>`;
    case "processing":
      return `<span class="buyer-order-status-badge buyer-status-processing">⚙️ Sedang Dikemas</span>`;
    case "shipped":
      return `<span class="buyer-order-status-badge buyer-status-shipped">🚚 Sedang Dikirim</span>`;
    case "delivered":
      return `<span class="buyer-order-status-badge buyer-status-delivered">📬 Pesanan Sampai</span>`;
    case "completed":
      return `<span class="buyer-order-status-badge buyer-status-completed">✅ Transaksi Selesai</span>`;
    case "cancelled":
      return `<span class="buyer-order-status-badge buyer-status-cancelled">❌ Dibatalkan</span>`;
    case "returned":
    case "return_requested":
      return `<span class="buyer-order-status-badge buyer-status-returned">↩️ Retur / Pengembalian</span>`;
    default:
      return `<span class="buyer-order-status-badge">${escapeHtml(status)}</span>`;
  }
}

async function fetchBuyerOrders() {
  const container = document.getElementById("buyer-orders-content");
  if (!container) return;
  const token = window.StoreState ? window.StoreState.buyerToken : window.buyerToken;
  if (!token) return;

  container.innerHTML = `<p style="text-align:center; color:var(--text-muted); padding:1.5rem">Memuat data pesanan...</p>`;

  try {
    const res = await fetch("/api/v1/buyer/orders", {
      headers: {
        "Authorization": `Bearer ${token}`
      }
    });

    if (!res.ok) {
      if (res.status === 401) {
        logoutBuyer();
        closeBuyerOrdersModal();
        showToast("Sesi masuk Anda telah berakhir. Silakan login kembali.", "warning");
        return;
      }
      const err = await res.json();
      container.innerHTML = `<p style="color:var(--rose); text-align:center; padding:1.5rem">Gagal memuat pesanan: ${escapeHtml(err.message || err.error || "Unknown error")}</p>`;
      return;
    }

    const orders = await res.json();
    if (!orders || orders.length === 0) {
      container.innerHTML = `
        <div style="text-align:center; padding:2.5rem 1rem; color:var(--text-muted)">
          <div style="font-size:2.5rem; margin-bottom:0.5rem">🛍️</div>
          <p style="font-weight:600; color:var(--text-heading); margin-bottom:0.3rem">Belum Ada Pesanan</p>
          <p style="font-size:0.85rem">Anda belum melakukan transaksi belanja apapun.</p>
        </div>
      `;
      return;
    }

    container.innerHTML = orders.map(o => {
      const s = (o.status || "").toLowerCase();
      let actionsHtml = "";

      if (s === "pending") {
        actionsHtml = `
          <div style="display:flex; gap:0.5rem; align-items:center">
            <button class="btn-order-cancel" onclick="handleBuyerCancelOrder('${o.id}')">❌ Batalkan Pesanan</button>
          </div>
        `;
      } else if (s === "shipped") {
        actionsHtml = `
          <div style="display:flex; gap:0.5rem; align-items:center">
            <button class="btn-order-confirm" onclick="handleBuyerConfirmDelivery('${o.id}')">✅ Konfirmasi Barang Diterima</button>
          </div>
        `;
      }

      let trackingHtml = "";
      if (o.tracking_number) {
        trackingHtml = `<div class="buyer-order-tracking">🚚 No. Resi: <strong>${escapeHtml(o.tracking_number)}</strong></div>`;
      }

      let cancelInfoHtml = "";
      if (s === "cancelled" && o.cancel_reason) {
        cancelInfoHtml = `<div style="font-size:0.75rem; color:var(--rose); margin-top:0.2rem">Alasan: ${escapeHtml(o.cancel_reason)}</div>`;
      }

      const orderDate = new Date(o.created_at).toLocaleString("id-ID", {
        day: "numeric",
        month: "short",
        year: "numeric",
        hour: "2-digit",
        minute: "2-digit"
      });

      return `
        <div class="buyer-order-card">
          <div class="buyer-order-header">
            <div>
              <span class="buyer-order-id">Order ID: #${escapeHtml(o.id.substring(0,8))}</span>
              <div style="font-size:0.75rem; color:var(--text-muted); margin-top:2px">${orderDate}</div>
            </div>
            <div>${getBuyerOrderStatusBadge(o.status)}</div>
          </div>
          <div class="buyer-order-body">
            <div>
              <div style="font-size:0.8rem; color:var(--text-muted)">Total Pembayaran:</div>
              <div class="buyer-order-amount">Rp ${o.total_amount.toLocaleString("id-ID")}</div>
              ${cancelInfoHtml}
            </div>
            <div>
              ${trackingHtml}
            </div>
          </div>
          ${actionsHtml ? `<div class="buyer-order-footer">${actionsHtml}</div>` : ""}
        </div>
      `;
    }).join("");

  } catch (e) {
    console.error("fetchBuyerOrders error:", e);
    container.innerHTML = `<p style="color:var(--rose); text-align:center; padding:1.5rem">Error: ${escapeHtml(e.message)}</p>`;
  }
}

async function handleBuyerCancelOrder(orderId) {
  const confirmCancel = await showConfirm("Apakah Anda yakin ingin membatalkan pesanan ini?", "Batalkan Pesanan", "⚠️");
  if (!confirmCancel) return;

  const reason = "Dibatalkan oleh pembeli";
  const token = window.StoreState ? window.StoreState.buyerToken : window.buyerToken;

  try {
    const res = await fetch(`/api/v1/orders/${orderId}/cancel`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Authorization": `Bearer ${token}`
      },
      body: JSON.stringify({ reason })
    });

    if (res.ok) {
      showToast("Pesanan berhasil dibatalkan.", "success");
      fetchBuyerOrders();
    } else {
      const err = await res.json();
      showToast(`Gagal membatalkan pesanan: ${err.message || err.error || JSON.stringify(err)}`, "error");
    }
  } catch (e) {
    showToast(`Error: ${e.message}`, "error");
  }
}

async function handleBuyerConfirmDelivery(orderId) {
  const confirmReceived = await showConfirm("Konfirmasi bahwa Anda telah menerima barang pesanan ini dalam kondisi baik?", "Konfirmasi Penerimaan", "📦");
  if (!confirmReceived) return;

  const token = window.StoreState ? window.StoreState.buyerToken : window.buyerToken;

  try {
    const res = await fetch(`/api/v1/orders/${orderId}/confirm-delivery`, {
      method: "POST",
      headers: {
        "Authorization": `Bearer ${token}`
      }
    });

    if (res.ok) {
      showToast("Terima kasih! Pesanan telah dikonfirmasi diterima.", "success");
      fetchBuyerOrders();
      if (typeof fetchBuyerOrdersDashboard === "function") {
        fetchBuyerOrdersDashboard();
      }
    } else {
      const err = await res.json();
      showToast(`Gagal konfirmasi pesanan: ${err.message || err.error || JSON.stringify(err)}`, "error");
    }
  } catch (e) {
    showToast(`Error: ${e.message}`, "error");
  }
}

// --- BUYER ORDERS DASHBOARD & ORDER DETAIL MODAL ---
let allDashboardOrders = [];
let currentDashboardOrderFilter = 'all';

async function fetchBuyerOrdersDashboard() {
  const container = document.getElementById("dashboard-orders-list");
  if (!container) return;
  const token = window.StoreState ? window.StoreState.buyerToken : window.buyerToken;
  if (!token) return;

  container.innerHTML = `<p style="text-align:center; color:var(--text-muted); padding:2rem">Memuat data pesanan...</p>`;

  try {
    const res = await buyerAuthFetch("/api/v1/buyer/orders");
    if (!res.ok) {
      if (res.status === 401) {
        logoutBuyer();
        showToast("Sesi masuk Anda telah berakhir. Silakan login kembali.", "warning");
        return;
      }
      const err = await res.json();
      container.innerHTML = `<p style="color:var(--rose); text-align:center; padding:2rem">Gagal memuat pesanan: ${escapeHtml(err.message || err.error || "Unknown error")}</p>`;
      return;
    }

    allDashboardOrders = await res.json();
    renderFilteredDashboardOrders();
  } catch (e) {
    console.error("fetchBuyerOrdersDashboard error:", e);
    container.innerHTML = `<p style="color:var(--rose); text-align:center; padding:2rem">Error: ${escapeHtml(e.message)}</p>`;
  }
}

function filterDashboardOrders(status, btnElement) {
  currentDashboardOrderFilter = status;
  if (btnElement) {
    const buttons = document.querySelectorAll(".order-filter-btn");
    buttons.forEach(b => b.classList.remove("active"));
    btnElement.classList.add("active");
  }
  renderFilteredDashboardOrders();
}

function renderFilteredDashboardOrders() {
  const container = document.getElementById("dashboard-orders-list");
  if (!container) return;

  let orders = allDashboardOrders || [];
  if (currentDashboardOrderFilter !== 'all') {
    orders = orders.filter(o => (o.status || "").toLowerCase() === currentDashboardOrderFilter.toLowerCase());
  }

  if (orders.length === 0) {
    container.innerHTML = `
      <div style="text-align:center; padding:3rem 1rem; color:var(--text-muted)">
        <div style="font-size:2.5rem; margin-bottom:0.5rem">📦</div>
        <p style="font-weight:600; color:var(--text-heading); margin-bottom:0.3rem">Tidak Ada Pesanan</p>
        <p style="font-size:0.85rem">Tidak ditemukan transaksi pesanan dengan status yang dipilih.</p>
      </div>
    `;
    return;
  }

  container.innerHTML = orders.map(o => {
    const s = (o.status || "").toLowerCase();
    let actionsHtml = `
      <button type="button" class="btn-sm-action" style="padding:0.4rem 0.8rem" onclick="openBuyerOrderDetailModal('${o.id}')">🔍 Rincian Pesanan</button>
    `;

    if (s === "pending") {
      actionsHtml += `
        <button type="button" class="btn-order-cancel" onclick="handleBuyerCancelOrder('${o.id}')">❌ Batalkan</button>
      `;
    } else if (s === "shipped") {
      actionsHtml += `
        <button type="button" class="btn-order-confirm" onclick="handleBuyerConfirmDelivery('${o.id}')">✅ Terima Barang</button>
      `;
    }

    let trackingHtml = "";
    if (o.tracking_number) {
      trackingHtml = `<div class="buyer-order-tracking">🚚 No. Resi: <strong>${escapeHtml(o.tracking_number)}</strong></div>`;
    }

    let cancelInfoHtml = "";
    if (s === "cancelled" && o.cancel_reason) {
      cancelInfoHtml = `<div style="font-size:0.75rem; color:var(--rose); margin-top:0.2rem">Alasan: ${escapeHtml(o.cancel_reason)}</div>`;
    }

    const orderDate = new Date(o.created_at).toLocaleString("id-ID", {
      day: "numeric",
      month: "short",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit"
    });

    const itemsSummary = (o.items || []).map(item => `
      <div style="display:flex; justify-content:space-between; font-size:0.85rem; padding:0.25rem 0; color:var(--text)">
        <span>${escapeHtml(item.product_name)} <span style="color:var(--text-muted)">x${item.quantity}</span></span>
        <span>Rp ${(item.unit_price * item.quantity).toLocaleString("id-ID")}</span>
      </div>
    `).join("");

    return `
      <div class="buyer-order-card">
        <div class="buyer-order-header">
          <div>
            <span class="buyer-order-id">Order ID: #${escapeHtml(o.id.substring(0,8))}</span>
            <div style="font-size:0.75rem; color:var(--text-muted); margin-top:2px">${orderDate}</div>
          </div>
          <div>${getBuyerOrderStatusBadge(o.status)}</div>
        </div>
        <div style="padding:0.4rem 0; border-bottom:1px solid rgba(255,255,255,0.04)">
          ${itemsSummary || '<p style="font-size:0.8rem; color:var(--text-muted)">1 Paket Produk</p>'}
        </div>
        <div class="buyer-order-body">
          <div>
            <div style="font-size:0.8rem; color:var(--text-muted)">Total Pembayaran:</div>
            <div class="buyer-order-amount">Rp ${o.total_amount.toLocaleString("id-ID")}</div>
            ${cancelInfoHtml}
          </div>
          <div>
            ${trackingHtml}
          </div>
        </div>
        <div class="buyer-order-footer">
          <div style="display:flex; gap:0.5rem; align-items:center; flex-wrap:wrap">
            ${actionsHtml}
          </div>
        </div>
      </div>
    `;
  }).join("");
}

// Detailed Order Modal
async function openBuyerOrderDetailModal(orderId) {
  const modal = document.getElementById("buyer-order-detail-modal");
  const container = document.getElementById("buyer-order-detail-content");
  if (!modal || !container) return;

  modal.style.display = "flex";
  container.innerHTML = `<p style="text-align:center; color:var(--text-muted); padding:2rem">Memuat rincian pesanan...</p>`;

  try {
    const res = await buyerAuthFetch(`/api/v1/buyer/orders/${orderId}`);
    if (!res.ok) {
      const err = await res.json();
      container.innerHTML = `<p style="color:var(--rose); text-align:center; padding:2rem">Gagal memuat detail pesanan: ${escapeHtml(err.message || "Tidak ditemukan")}</p>`;
      return;
    }

    const o = await res.json();
    const s = (o.status || "").toLowerCase();

    const orderDate = new Date(o.created_at).toLocaleString("id-ID", {
      day: "numeric",
      month: "long",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit"
    });

    const itemsHtml = (o.items || []).map(item => `
      <div style="display:flex; justify-content:space-between; align-items:center; padding:0.6rem 0; border-bottom:1px solid var(--card-border)">
        <div>
          <div style="font-weight:600; font-size:0.9rem; color:var(--text-heading)">${escapeHtml(item.product_name)}</div>
          <div style="font-size:0.75rem; color:var(--text-muted)">SKU: ${escapeHtml(item.product_id.substring(0,8))} | Qty: ${item.quantity} x Rp ${item.unit_price.toLocaleString("id-ID")}</div>
        </div>
        <div style="font-weight:700; color:var(--text-heading); font-size:0.95rem">
          Rp ${(item.unit_price * item.quantity).toLocaleString("id-ID")}
        </div>
      </div>
    `).join("");

    let addressHtml = `<p style="font-size:0.85rem; color:var(--text)">${escapeHtml(o.shipping_address)}</p>`;
    if (o.shipping_snapshot) {
      const snap = o.shipping_snapshot;
      addressHtml = `
        <div style="font-size:0.85rem; line-height:1.4">
          <div style="font-weight:700; color:var(--text-heading)">${escapeHtml(snap.recipient_name)} (${escapeHtml(snap.phone_number)})</div>
          <div style="color:var(--text); margin-top:0.2rem">${escapeHtml(snap.street_address)}</div>
          <div style="color:var(--text-muted)">Kec. ${escapeHtml(snap.subdistrict)}, ${escapeHtml(snap.city)}, ${escapeHtml(snap.province)} ${escapeHtml(snap.postal_code)}</div>
        </div>
      `;
    }

    let actionsHtml = "";
    if (s === "pending") {
      actionsHtml = `
        <button type="button" class="btn-order-cancel" onclick="closeBuyerOrderDetailModal(); handleBuyerCancelOrder('${o.id}')">❌ Batalkan Pesanan Ini</button>
      `;
    } else if (s === "shipped") {
      actionsHtml = `
        <button type="button" class="btn-order-confirm" onclick="closeBuyerOrderDetailModal(); handleBuyerConfirmDelivery('${o.id}')">✅ Konfirmasi Barang Diterima</button>
      `;
    }

    container.innerHTML = `
      <div style="display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:0.5rem; border-bottom:1px solid var(--card-border); padding-bottom:0.75rem">
        <div>
          <div style="font-family:monospace; font-size:0.9rem; color:var(--text-muted)">ID: #${escapeHtml(o.id)}</div>
          <div style="font-size:0.8rem; color:var(--text-muted); margin-top:0.2rem">Waktu Pemesanan: ${orderDate}</div>
        </div>
        <div>${getBuyerOrderStatusBadge(o.status)}</div>
      </div>

      <div>
        <h4 style="font-size:0.9rem; color:var(--text-muted); margin-bottom:0.4rem; text-transform:uppercase; letter-spacing:0.5px">📍 Alamat Pengiriman (Snapshot Permanen)</h4>
        <div style="background:rgba(255,255,255,0.02); border:1px solid var(--card-border); border-radius:8px; padding:0.75rem">
          ${addressHtml}
        </div>
      </div>

      ${o.tracking_number ? `
        <div>
          <h4 style="font-size:0.9rem; color:var(--text-muted); margin-bottom:0.4rem; text-transform:uppercase; letter-spacing:0.5px">🚚 Pelacakan Pengiriman</h4>
          <div class="buyer-order-tracking" style="padding:0.6rem 0.8rem; font-size:0.9rem">
            Nomor Resi: <strong>${escapeHtml(o.tracking_number)}</strong>
          </div>
        </div>
      ` : ""}

      ${o.cancel_reason ? `
        <div>
          <h4 style="font-size:0.9rem; color:var(--rose); margin-bottom:0.4rem; text-transform:uppercase; letter-spacing:0.5px">Alasan Pembatalan</h4>
          <div style="background:rgba(244,63,94,0.08); border:1px solid rgba(244,63,94,0.25); border-radius:8px; padding:0.75rem; color:#fda4af; font-size:0.85rem">
            ${escapeHtml(o.cancel_reason)}
          </div>
        </div>
      ` : ""}

      <div>
        <h4 style="font-size:0.9rem; color:var(--text-muted); margin-bottom:0.4rem; text-transform:uppercase; letter-spacing:0.5px">🛍️ Rincian Produk</h4>
        <div style="background:rgba(255,255,255,0.02); border:1px solid var(--card-border); border-radius:8px; padding:0 0.75rem">
          ${itemsHtml}
        </div>
      </div>

      <div style="display:flex; justify-content:space-between; align-items:center; padding:0.75rem; background:rgba(238,77,45,0.08); border:1px solid rgba(238,77,45,0.2); border-radius:8px">
        <span style="font-weight:700; font-size:1rem; color:var(--text-heading)">Total Pembayaran:</span>
        <span style="font-weight:800; font-size:1.2rem; color:var(--shopee-orange)">Rp ${o.total_amount.toLocaleString("id-ID")}</span>
      </div>

      ${actionsHtml ? `
        <div style="display:flex; justify-content:flex-end; gap:0.75rem; padding-top:0.5rem">
          ${actionsHtml}
        </div>
      ` : ""}
    `;
  } catch (e) {
    console.error("openBuyerOrderDetailModal error:", e);
    container.innerHTML = `<p style="color:var(--rose); text-align:center; padding:2rem">Error: ${escapeHtml(e.message)}</p>`;
  }
}

function closeBuyerOrderDetailModal() {
  const modal = document.getElementById("buyer-order-detail-modal");
  if (modal) modal.style.display = "none";
}

// Window Exports
window.escapeHtml = escapeHtml;
window.fetchBuyerAddresses = fetchBuyerAddresses;
window.renderBuyerAddressesList = renderBuyerAddressesList;
window.renderDashboardAddressesList = renderDashboardAddressesList;
window.fetchBuyerAddressesDashboard = fetchBuyerAddressesDashboard;
window.openAddAddressModal = openAddAddressModal;
window.closeBuyerAddressModal = closeBuyerAddressModal;
window.handleSaveBuyerAddress = handleSaveBuyerAddress;
window.handleSetDefaultAddress = handleSetDefaultAddress;
window.handleDeleteAddress = handleDeleteAddress;
window.renderConfirmedAddressInCheckout = renderConfirmedAddressInCheckout;
window.switchCheckoutAddress = switchCheckoutAddress;
window.handleProcessCheckout = handleProcessCheckout;
window.initiateMidtransPayment = initiateMidtransPayment;
window.openBuyerOrdersModal = openBuyerOrdersModal;
window.closeBuyerOrdersModal = closeBuyerOrdersModal;
window.getBuyerOrderStatusBadge = getBuyerOrderStatusBadge;
window.fetchBuyerOrders = fetchBuyerOrders;
window.handleBuyerCancelOrder = handleBuyerCancelOrder;
window.handleBuyerConfirmDelivery = handleBuyerConfirmDelivery;
window.fetchBuyerOrdersDashboard = fetchBuyerOrdersDashboard;
window.filterDashboardOrders = filterDashboardOrders;
window.renderFilteredDashboardOrders = renderFilteredDashboardOrders;
window.openBuyerOrderDetailModal = openBuyerOrderDetailModal;
window.closeBuyerOrderDetailModal = closeBuyerOrderDetailModal;

