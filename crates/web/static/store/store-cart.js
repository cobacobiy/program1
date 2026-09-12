/* ==========================================================================
   STOREFRONT CART ENGINE
   File: /crates/web/static/store/store-cart.js
   ========================================================================== */

function addToCart(productId, event) {
  const catalog = window.StoreState ? window.StoreState.catalog : (window.catalog || []);
  let cart = window.StoreState ? window.StoreState.cart : (window.cart || []);

  const item = catalog.find(p => p.id === productId);
  if (!item) return;

  const existing = cart.find(c => c.product_id === productId);
  if (existing) {
    existing.quantity += 1;
  } else {
    cart.push({ product_id: productId, name: item.name, price: item.price, quantity: 1 });
  }

  if (window.StoreState) window.StoreState.cart = cart;
  window.cart = cart;

  updateCartUI();

  // Instant tactile button feedback
  let btn = event ? (event.currentTarget || event.target) : null;
  if (!btn || !btn.classList) {
    btn = document.activeElement;
  }
  if (btn && btn.classList && (btn.classList.contains('btn-add-cart') || btn.classList.contains('btn-add'))) {
    btn.classList.add('added');
    const prevHtml = btn.innerHTML;
    btn.innerHTML = '✓ Ditambahkan!';
    btn.style.pointerEvents = 'none';
    setTimeout(() => {
      btn.classList.remove('added');
      btn.innerHTML = prevHtml || '+ Beli';
      btn.style.pointerEvents = '';
    }, 1000);
  }

  // Animate floating cart badge & trigger
  const cartBadge = document.getElementById('cart-count');
  const cartTrigger = document.querySelector('.cart-trigger');
  if (cartBadge) {
    cartBadge.classList.remove('cart-bounce');
    void cartBadge.offsetWidth;
    cartBadge.classList.add('cart-bounce');
  }
  if (cartTrigger) {
    cartTrigger.classList.remove('cart-shake');
    void cartTrigger.offsetWidth;
    cartTrigger.classList.add('cart-shake');
  }

  if (typeof showToast === 'function') {
    showToast(`"${item.name}" berhasil ditambahkan ke keranjang!`, 'success');
  }
}

function updateQuantity(productId, delta) {
  let cart = window.StoreState ? window.StoreState.cart : (window.cart || []);
  const item = cart.find(c => c.product_id === productId);
  if (!item) return;

  item.quantity += delta;
  if (item.quantity <= 0) {
    cart = cart.filter(c => c.product_id !== productId);
  }

  if (window.StoreState) window.StoreState.cart = cart;
  window.cart = cart;

  updateCartUI();
}

function updateCartUI() {
  const container = document.getElementById('cart-items-container');
  const checkoutSection = document.getElementById('checkout-section');
  const unauthBox = document.getElementById('checkout-unauth-box');
  const unverifiedBox = document.getElementById('checkout-unverified-box');
  const confirmedForm = document.getElementById('checkout-confirmed-form');
  const badge = document.getElementById('cart-count');

  const cart = window.StoreState ? window.StoreState.cart : (window.cart || []);
  const buyerToken = window.StoreState ? window.StoreState.buyerToken : window.buyerToken;
  const activeBuyer = window.StoreState ? window.StoreState.activeBuyer : window.activeBuyer;

  const count = cart.reduce((acc, i) => acc + i.quantity, 0);
  if (badge) badge.innerText = count;

  if (!container || !checkoutSection) return;

  if (cart.length === 0) {
    container.innerHTML = '<p style="color:var(--text-muted); text-align:center; padding:1.5rem">Keranjang belanja Anda kosong.</p>';
    checkoutSection.style.display = 'none';
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

  checkoutSection.style.display = 'block';

  // Evaluate Buyer Authentication & Phone Verification Gates
  if (!buyerToken || !activeBuyer) {
    if (unauthBox) unauthBox.style.display = 'block';
    if (unverifiedBox) unverifiedBox.style.display = 'none';
    if (confirmedForm) confirmedForm.style.display = 'none';
  } else if (!activeBuyer.phone_verified) {
    if (unauthBox) unauthBox.style.display = 'none';
    if (unverifiedBox) unverifiedBox.style.display = 'block';
    if (confirmedForm) confirmedForm.style.display = 'none';
  } else {
    if (unauthBox) unauthBox.style.display = 'none';
    if (unverifiedBox) unverifiedBox.style.display = 'none';
    if (confirmedForm) confirmedForm.style.display = 'block';
    if (typeof renderConfirmedAddressInCheckout === "function") {
      renderConfirmedAddressInCheckout();
    }
  }
}

function openCart() {
  const sidebar = document.getElementById("cart-sidebar");
  if (sidebar) sidebar.classList.add("open");
}

function closeCart() {
  const sidebar = document.getElementById("cart-sidebar");
  if (sidebar) sidebar.classList.remove("open");
}

window.addToCart = addToCart;
window.updateQuantity = updateQuantity;
window.updateCartUI = updateCartUI;
window.openCart = openCart;
window.closeCart = closeCart;
