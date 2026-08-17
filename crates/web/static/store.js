/* ==========================================================================
   STOREFRONT LOGIC ENGINE (SHOPEE-STYLE RESPONSIVE WITH BUYER LOGIN)
   Target File: /home/cacyos/Downloads/github/program1/crates/web/static/store.js
   ========================================================================== */

let cart = [];
let catalog = [];
let activeCategory = "ALL";
let searchQuery = "";
let activeBuyer = null;

// Initialize Storefront
async function initStore() {
  try {
    // 1. Check Storefront Buyer Session
    checkBuyerSession();

    // 2. Fetch Store Information
    const infoRes = await fetch('/api/v1/store/info');
    if (infoRes.ok) {
      const info = await infoRes.json();
      const nameEl = document.getElementById('header-store-name');
      if (nameEl) nameEl.innerText = info.store_name || "AURA Storefront";
      
      const titleEl = document.getElementById('page-title');
      if (titleEl) titleEl.innerText = `${info.store_name || "AURA Storefront"} — Shopee Official Store`;
    }

    // 3. Fetch Catalog Products
    const res = await fetch('/api/v1/catalog');
    if (res.ok) {
      catalog = await res.json();
      renderCategoryPills();
      renderCatalog();
    }

    // 4. Start Flash Sale Countdown Timer
    startFlashSaleTimer();

    // 5. Setup Event Listeners
    setupSearchListener();
    setupBuyerLoginForm();
  } catch (e) {
    console.error("Failed to initialize storefront data:", e);
  }
}

// --- BUYER LOGIN & SESSION MANAGEMENT ---
function checkBuyerSession() {
  const saved = localStorage.getItem("shopee_buyer_session");
  if (saved) {
    try {
      activeBuyer = JSON.parse(saved);
      renderBuyerHeaderState();
    } catch (e) {
      localStorage.removeItem("shopee_buyer_session");
    }
  } else {
    renderBuyerHeaderState();
  }
}

function renderBuyerHeaderState() {
  const btnLogin = document.getElementById("btn-buyer-login");
  const badgeProfile = document.getElementById("buyer-profile-badge");
  const avatarEl = document.getElementById("buyer-avatar");
  const nameLabelEl = document.getElementById("buyer-name-label");

  if (activeBuyer) {
    if (btnLogin) btnLogin.style.display = "none";
    if (badgeProfile) badgeProfile.style.display = "flex";
    if (avatarEl) avatarEl.innerText = activeBuyer.name.charAt(0).toUpperCase();
    if (nameLabelEl) nameLabelEl.innerText = activeBuyer.name;

    // Auto-fill Checkout Form
    const custNameInput = document.getElementById("cust-name");
    const custEmailInput = document.getElementById("cust-email");
    if (custNameInput && !custNameInput.value) custNameInput.value = activeBuyer.name;
    if (custEmailInput && !custEmailInput.value) custEmailInput.value = activeBuyer.email;
  } else {
    if (btnLogin) btnLogin.style.display = "block";
    if (badgeProfile) badgeProfile.style.display = "none";
  }
}

function openBuyerLoginModal() {
  const modal = document.getElementById("buyer-login-modal");
  const loggedView = document.getElementById("buyer-logged-in-view");
  const loginForm = document.getElementById("buyer-login-form");

  if (activeBuyer) {
    if (loggedView) loggedView.style.display = "block";
    if (loginForm) loginForm.style.display = "none";
    document.getElementById("modal-buyer-avatar").innerText = activeBuyer.name.charAt(0).toUpperCase();
    document.getElementById("modal-buyer-name").innerText = activeBuyer.name;
    document.getElementById("modal-buyer-email").innerText = activeBuyer.email;
  } else {
    if (loggedView) loggedView.style.display = "none";
    if (loginForm) loginForm.style.display = "block";
  }

  if (modal) modal.style.display = "flex";
}

function closeBuyerLoginModal() {
  const modal = document.getElementById("buyer-login-modal");
  if (modal) modal.style.display = "none";
}

function quickGuestLoginBuyer() {
  const sampleBuyer = {
    id: "buyer-" + Date.now(),
    name: "Budi Santoso (Pembeli)",
    email: "budi.santoso@shopee.co.id",
    phone: "081234567890"
  };
  loginBuyerSuccess(sampleBuyer);
}

function loginBuyerSuccess(buyerObj) {
  activeBuyer = buyerObj;
  localStorage.setItem("shopee_buyer_session", JSON.stringify(buyerObj));
  renderBuyerHeaderState();
  closeBuyerLoginModal();
  alert(`🎉 Selamat datang kembali, ${buyerObj.name}! Akun pembeli berhasil masuk.`);
}

function logoutBuyer() {
  activeBuyer = null;
  localStorage.removeItem("shopee_buyer_session");
  renderBuyerHeaderState();
  closeBuyerLoginModal();
  alert("👋 Akun pembeli berhasil keluar (logged out).");
}

function setupBuyerLoginForm() {
  const form = document.getElementById("buyer-login-form");
  if (form) {
    form.onsubmit = (e) => {
      e.preventDefault();
      const loginInput = document.getElementById("buyer-input-login").value;
      const passInput = document.getElementById("buyer-input-pass").value;

      let buyerName = loginInput.split("@")[0];
      buyerName = buyerName.charAt(0).toUpperCase() + buyerName.slice(1);

      const buyerObj = {
        id: "buyer-" + Date.now(),
        name: buyerName.includes("08") ? "Pembeli WA (" + loginInput + ")" : buyerName,
        email: loginInput.includes("@") ? loginInput : loginInput + "@buyer.shopee.co.id",
        phone: loginInput
      };
      loginBuyerSuccess(buyerObj);
    };
  }
}

// Flash Sale Countdown Timer Logic
function startFlashSaleTimer() {
  let seconds = 3 * 3600 + 45 * 60 + 12; // 03:45:12
  const hoursEl = document.getElementById('timer-hours');
  const minsEl = document.getElementById('timer-mins');
  const secsEl = document.getElementById('timer-secs');

  if (!hoursEl || !minsEl || !secsEl) return;

  setInterval(() => {
    if (seconds <= 0) seconds = 24 * 3600;
    seconds--;

    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;

    hoursEl.innerText = String(h).padStart(2, '0');
    minsEl.innerText = String(m).padStart(2, '0');
    secsEl.innerText = String(s).padStart(2, '0');
  }, 1000);
}

// Category Pills Generator
function renderCategoryPills() {
  const container = document.getElementById('category-bar');
  if (!container) return;

  const categories = ["ALL", ...new Set(catalog.map(p => p.category))];
  
  container.innerHTML = categories.map(cat => `
    <button class="category-pill ${cat === activeCategory ? 'active' : ''}" onclick="filterCategory('${cat}')">
      ${cat === 'ALL' ? '🔥 Semua Produk' : cat}
    </button>
  `).join('');
}

function filterCategory(cat) {
  activeCategory = cat;
  renderCategoryPills();
  renderCatalog();
}

function setupSearchListener() {
  const input = document.getElementById('search-input');
  if (input) {
    input.addEventListener('input', (e) => {
      searchQuery = e.target.value.toLowerCase().trim();
      renderCatalog();
    });
  }
}

// Render Products Grid (Shopee-Style Card)
function renderCatalog() {
  const grid = document.getElementById('product-grid');
  if (!grid) return;

  let filtered = catalog;
  if (activeCategory !== 'ALL') {
    filtered = filtered.filter(p => p.category === activeCategory);
  }

  if (searchQuery) {
    filtered = filtered.filter(p => 
      p.name.toLowerCase().includes(searchQuery) || 
      p.description.toLowerCase().includes(searchQuery) ||
      p.sku.toLowerCase().includes(searchQuery)
    );
  }

  if (filtered.length === 0) {
    grid.innerHTML = '<div style="grid-column:1/-1; text-align:center; padding:3rem; color:var(--text-muted)">Produk tidak ditemukan.</div>';
    return;
  }

  grid.innerHTML = filtered.map(p => `
    <div class="product-card">
      <span class="discount-badge">OFF 15%</span>
      <img src="${p.image_url}" alt="${p.name}" class="product-img" loading="lazy">
      <div class="product-info">
        <span class="product-category">${p.category}</span>
        <h4 class="product-name">${p.name}</h4>
        <p class="product-desc">${p.description || ""}</p>
        <div class="product-rating">
          ★ ★ ★ ★ ★ <span style="color:var(--text-muted); font-size:0.7rem">(4.9 | 1.2k Terjual)</span>
        </div>
        <div class="product-footer">
          <span class="product-price">Rp ${p.price.toLocaleString('id-ID')}</span>
          <button class="btn-add-cart" onclick="addToCart('${p.id}')">+ Beli</button>
        </div>
      </div>
    </div>
  `).join('');
}

// Shopping Cart State & UI Controls
function addToCart(productId) {
  const item = catalog.find(p => p.id === productId);
  if (!item) return;

  const existing = cart.find(c => c.product_id === productId);
  if (existing) {
    existing.quantity += 1;
  } else {
    cart.push({ product_id: productId, name: item.name, price: item.price, quantity: 1 });
  }

  updateCartUI();
}

function updateQuantity(productId, delta) {
  const item = cart.find(c => c.product_id === productId);
  if (!item) return;

  item.quantity += delta;
  if (item.quantity <= 0) {
    cart = cart.filter(c => c.product_id !== productId);
  }
  updateCartUI();
}

function updateCartUI() {
  const totalCount = cart.reduce((acc, i) => acc + i.quantity, 0);
  const badge = document.getElementById('cart-count');
  if (badge) badge.innerText = totalCount;

  const container = document.getElementById('cart-items-container');
  const form = document.getElementById('checkout-form');
  if (!container || !form) return;

  if (cart.length === 0) {
    container.innerHTML = '<p style="color:var(--text-muted); text-align:center; padding:1.5rem">Keranjang belanja Anda kosong.</p>';
    form.style.display = 'none';
    return;
  }

  let total = 0;
  container.innerHTML = cart.map(i => {
    const itemTotal = i.price * i.quantity;
    total += itemTotal;
    return `
      <div class="cart-item">
        <div>
          <strong style="color:#fff">${i.name}</strong>
          <div style="font-size:0.8rem; color:var(--text-muted)">Rp ${i.price.toLocaleString('id-ID')}</div>
          <div class="cart-qty-controls">
            <button class="btn-qty" onclick="updateQuantity('${i.product_id}', -1)">-</button>
            <span style="font-size:0.85rem; font-weight:700; padding:0 0.4rem">${i.quantity}</span>
            <button class="btn-qty" onclick="updateQuantity('${i.product_id}', 1)">+</button>
          </div>
        </div>
        <span style="font-weight:700; color:var(--shopee-orange)">Rp ${itemTotal.toLocaleString('id-ID')}</span>
      </div>
    `;
  }).join('') + `
    <div style="display:flex; justify-content:space-between; margin-top:1rem; font-size:1.1rem; font-weight:700">
      <span style="color:#fff">Total Pembayaran:</span>
      <span style="color:var(--shopee-orange)">Rp ${total.toLocaleString('id-ID')}</span>
    </div>
  `;

  form.style.display = 'block';
}

function openCart() {
  const modal = document.getElementById('cart-modal');
  if (modal) modal.style.display = 'flex';
}

function closeCart() {
  const modal = document.getElementById('cart-modal');
  if (modal) modal.style.display = 'none';
}

// Form Submission (Checkout Order)
document.addEventListener("DOMContentLoaded", () => {
  const checkoutForm = document.getElementById('checkout-form');
  if (checkoutForm) {
    checkoutForm.onsubmit = async (e) => {
      e.preventDefault();
      const customer_name = document.getElementById('cust-name').value;
      const customer_email = document.getElementById('cust-email').value;
      const shipping_address = document.getElementById('cust-address').value;

      const items = cart.map(i => ({ product_id: i.product_id, quantity: i.quantity }));

      const res = await fetch('/api/v1/orders', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ customer_name, customer_email, shipping_address, items })
      });

      if (res.ok) {
        alert('🎉 Pesanan Berhasil Dibuat! Order Anda terhubung langsung ke Ginee Hub OMS.');
        cart = [];
        updateCartUI();
        closeCart();
      } else {
        const err = await res.json();
        alert(`Checkout Gagal: ${err.error || JSON.stringify(err)}`);
      }
    };
  }

  initStore();
});
