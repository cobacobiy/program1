/* ==========================================================================
   STOREFRONT PRODUCT CATALOG & SEARCH ENGINE
   File: /crates/web/static/store/store-catalog.js
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
async function renderCategoryPills() {
  const container = document.getElementById('category-bar');
  if (!container) return;

  const catalog = window.StoreState ? window.StoreState.catalog : (window.catalog || []);
  const activeCategory = window.StoreState ? window.StoreState.activeCategory : (window.activeCategory || "ALL");

  let categoriesList = window.StoreState ? window.StoreState.categoriesList : window.categoriesList;
  if (!categoriesList) {
    try {
      const res = await fetch('/api/v1/categories');
      if (res.ok) {
        categoriesList = await res.json();
        if (window.StoreState) window.StoreState.categoriesList = categoriesList;
        window.categoriesList = categoriesList;
      }
    } catch (e) {
      // fallback
    }
  }

  if (categoriesList && categoriesList.length > 0) {
    const allCount = categoriesList.reduce((s, c) => s + (c.product_count || 0), 0);
    let html = `
      <button class="category-pill ${activeCategory === 'ALL' ? 'active' : ''}" onclick="filterCategory('ALL')">
        🔥 Semua Produk (${allCount})
      </button>
    `;
    html += categoriesList.map(cat => `
      <button class="category-pill ${cat.name === activeCategory || cat.slug === activeCategory ? 'active' : ''}" onclick="filterCategory('${cat.name}')">
        ${cat.icon || '🏷️'} ${cat.name} (${cat.product_count || 0})
      </button>
    `).join('');
    container.innerHTML = html;
    return;
  }

  const categories = ["ALL", ...new Set(catalog.map(p => p.category))];

  container.innerHTML = categories.map(cat => `
    <button class="category-pill ${cat === activeCategory ? 'active' : ''}" onclick="filterCategory('${cat}')">
      ${cat === 'ALL' ? '🔥 Semua Produk' : cat}
    </button>
  `).join('');
}

function filterCategory(cat) {
  if (window.StoreState) window.StoreState.activeCategory = cat;
  window.activeCategory = cat;
  renderCategoryPills();
  renderCatalog();
}

function setupSearchListener() {
  const input = document.getElementById('search-input');
  if (input) {
    input.addEventListener('input', (e) => {
      const q = e.target.value.toLowerCase().trim();
      if (window.StoreState) window.StoreState.searchQuery = q;
      window.searchQuery = q;
      renderCatalog();
    });
  }
}

// Render Products Grid (Shopee-Style Card with Wishlist & Variant Selector)
function renderCatalog() {
  const grid = document.getElementById('product-grid');
  if (!grid) return;

  const catalog = window.StoreState ? window.StoreState.catalog : (window.catalog || []);
  const activeCategory = window.StoreState ? window.StoreState.activeCategory : (window.activeCategory || "ALL");
  const searchQuery = window.StoreState ? window.StoreState.searchQuery : (window.searchQuery || "");

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
    <div class="product-card" onclick="showProductDetail('${p.id}', event)" style="cursor:pointer">
      <button type="button" class="wishlist-btn" data-product-id="${p.id}" onclick="toggleWishlist('${p.id}', event)" title="Favorit">
        <span class="wishlist-icon">♡</span>
      </button>
      <span class="discount-badge">OFF 15%</span>
      <img src="${escapeHtml(p.image_url)}" alt="${escapeHtml(p.name)}" class="product-img" loading="lazy" onerror="this.src='https://placehold.co/400'">
      <div class="product-info">
        <span class="product-category">${escapeHtml(p.category)}</span>
        <h4 class="product-name">${escapeHtml(p.name)}</h4>
        <p class="product-desc">${escapeHtml(p.description || "")}</p>
        <div class="product-rating">
          ★ ★ ★ ★ ★ <span style="color:var(--text-muted); font-size:0.7rem">(4.9 | 1.2k Terjual)</span>
        </div>
        <div class="product-footer">
          <span class="product-price">Rp ${p.price.toLocaleString('id-ID')}</span>
          <button type="button" class="btn-add-cart" onclick="event.stopPropagation(); showProductDetail('${p.id}', event)">+ Beli</button>
        </div>
      </div>
    </div>
  `).join('');

  updateWishlistButtons();
}

// --- WISHLIST ENGINE ---
async function toggleWishlist(productId, event) {
  if (event) {
    event.stopPropagation();
    event.preventDefault();
  }

  const token = window.StoreState ? window.StoreState.buyerToken : window.buyerToken;
  if (!token) {
    if (typeof showToast === 'function') {
      showToast('Silakan masuk terlebih dahulu untuk menyimpan wishlist ❤️', 'warning');
    }
    if (typeof openBuyerLoginModal === 'function') {
      openBuyerLoginModal();
    } else if (typeof switchStoreTab === 'function') {
      switchStoreTab('profile');
    }
    return;
  }

  const btn = event ? event.currentTarget : document.querySelector(`.wishlist-btn[data-product-id="${productId}"]`);
  const icon = btn ? btn.querySelector('.wishlist-icon') : null;
  const isActive = btn ? btn.classList.contains('active') : false;

  try {
    if (isActive) {
      // DELETE — remove from wishlist
      const res = await fetch(`/api/v1/buyer/wishlist/${productId}`, {
        method: 'DELETE',
        headers: { 'Authorization': `Bearer ${token}` }
      });
      if (res.ok || res.status === 204) {
        if (btn) btn.classList.remove('active');
        if (icon) icon.textContent = '♡';
        if (typeof showToast === 'function') {
          showToast('Dihapus dari wishlist', 'info');
        }
      }
    } else {
      // POST — add to wishlist
      const res = await fetch(`/api/v1/buyer/wishlist/${productId}`, {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${token}` }
      });
      if (res.ok || res.status === 201 || res.status === 409) {
        if (btn) btn.classList.add('active');
        if (icon) icon.textContent = '❤️';
        if (typeof showToast === 'function') {
          showToast('Ditambahkan ke wishlist! ❤️', 'success');
        }
      } else {
        const err = await res.json().catch(() => ({}));
        if (typeof showToast === 'function') {
          showToast(`Gagal: ${err.message || 'Error'}`, 'error');
        }
      }
    }
  } catch (e) {
    console.error('Wishlist error:', e);
  }
}

async function updateWishlistButtons() {
  const token = window.StoreState ? window.StoreState.buyerToken : window.buyerToken;
  if (!token) return;

  try {
    const res = await fetch('/api/v1/buyer/wishlist', {
      headers: { 'Authorization': `Bearer ${token}` }
    });
    if (res.ok) {
      const items = await res.json();
      const set = new Set(items.map(i => i.product_id));
      document.querySelectorAll('.wishlist-btn').forEach(btn => {
        const pid = btn.dataset.productId;
        const icon = btn.querySelector('.wishlist-icon');
        if (set.has(pid)) {
          btn.classList.add('active');
          if (icon) icon.textContent = '❤️';
        } else {
          btn.classList.remove('active');
          if (icon) icon.textContent = '♡';
        }
      });
    }
  } catch (e) {
    console.debug('Failed to sync wishlist buttons:', e);
  }
}

// --- PRODUCT DETAIL & VARIANT SELECTOR ENGINE ---
async function showProductDetail(productId, event) {
  if (event) {
    event.stopPropagation();
  }

  const catalog = window.StoreState ? window.StoreState.catalog : (window.catalog || []);
  const product = catalog.find(p => p.id === productId);
  if (!product) return;

  let variants = [];
  try {
    const res = await fetch(`/api/v1/catalog/${productId}/variants`);
    if (res.ok) {
      variants = await res.json();
    }
  } catch (e) {
    console.warn('Gagal fetch variants:', e);
  }

  let modal = document.getElementById('product-detail-modal');
  if (!modal) {
    modal = document.createElement('div');
    modal.id = 'product-detail-modal';
    modal.className = 'product-detail-overlay';
    document.body.appendChild(modal);
  }

  const hasVariants = variants && variants.length > 0;
  const variantSection = hasVariants ? `
    <div class="variant-section">
      <h4 style="margin:1rem 0 0.5rem; font-size:0.95rem; color:var(--text-heading)">Pilih Varian:</h4>
      ${renderVariantOptions(variants, product)}
    </div>
  ` : '';

  modal.innerHTML = `
    <div class="product-detail-content">
      <button class="close-detail" onclick="closeProductDetail()">&times;</button>
      <img src="${escapeHtml(product.image_url)}" alt="${escapeHtml(product.name)}" class="detail-img" onerror="this.src='https://placehold.co/400'">
      <span class="product-category" style="margin-bottom:0.4rem; display:inline-block">${escapeHtml(product.category)}</span>
      <h3 style="margin:0 0 0.5rem 0; font-size:1.25rem; color:var(--text-heading)">${escapeHtml(product.name)}</h3>
      <div class="detail-price" id="detail-modal-price" style="font-size:1.3rem; font-weight:700; color:var(--shopee-orange); margin-bottom:0.75rem">
        Rp ${product.price.toLocaleString('id-ID')}
      </div>
      <p class="detail-desc" style="font-size:0.85rem; color:var(--text-muted); line-height:1.4; margin-bottom:1rem">
        ${escapeHtml(product.description || "Tidak ada deskripsi.")}
      </p>
      ${variantSection}
      <div style="display:flex; gap:0.5rem; margin-top:1.5rem">
        <button class="btn-add-cart detail-add-btn" style="flex:1; padding:0.75rem" onclick="addToCartWithVariant('${productId}', event)">
          🛒 + Tambah ke Keranjang
        </button>
      </div>
    </div>
  `;
  modal.style.display = 'flex';

  // Auto-select first pill in each group if available
  document.querySelectorAll('.variant-group').forEach(group => {
    const firstPill = group.querySelector('.variant-pill');
    if (firstPill) firstPill.click();
  });
}

function renderVariantOptions(variants, product) {
  const groups = {};
  variants.forEach(v => {
    if (!groups[v.variant_name]) groups[v.variant_name] = [];
    groups[v.variant_name].push(v);
  });

  return Object.entries(groups).map(([name, items]) => `
    <div class="variant-group" data-group-name="${escapeHtml(name)}">
      <label style="font-weight:600; font-size:0.85rem; margin-bottom:0.3rem; display:block; color:var(--text-heading)">${escapeHtml(name)}:</label>
      <div class="variant-options">
        ${items.map(v => `
          <button type="button" class="variant-pill"
                  data-variant-id="${v.id}"
                  data-variant-name="${escapeHtml(v.variant_name)}"
                  data-variant-value="${escapeHtml(v.variant_value)}"
                  data-variant-price="${v.price_override || product.price}"
                  data-stock="${v.stock_quantity}"
                  onclick="selectVariant(this)">
            <span>${escapeHtml(v.variant_value)}</span>
            ${v.price_override ? `<small style="display:block; font-size:0.7rem; opacity:0.8">Rp ${v.price_override.toLocaleString('id-ID')}</small>` : ''}
          </button>
        `).join('')}
      </div>
    </div>
  `).join('');
}

function selectVariant(btn) {
  const parent = btn.closest('.variant-options');
  if (parent) {
    parent.querySelectorAll('.variant-pill').forEach(b => b.classList.remove('selected'));
  }
  btn.classList.add('selected');

  // Update detail modal price
  const price = parseFloat(btn.dataset.variantPrice);
  const priceEl = document.getElementById('detail-modal-price');
  if (priceEl && !isNaN(price)) {
    priceEl.innerText = `Rp ${price.toLocaleString('id-ID')}`;
  }
}

function addToCartWithVariant(productId, event) {
  const variantGroups = document.querySelectorAll('.variant-group');
  const selectedPills = document.querySelectorAll('.variant-pill.selected');

  if (variantGroups.length > 0 && selectedPills.length < variantGroups.length) {
    if (typeof showToast === 'function') {
      showToast('Silakan pilih semua varian terlebih dahulu!', 'warning');
    } else {
      alert('Silakan pilih semua varian terlebih dahulu!');
    }
    return;
  }

  let variantId = null;
  let variantLabels = [];
  let overridePrice = null;

  selectedPills.forEach(pill => {
    variantId = pill.dataset.variantId; // Use selected variant
    variantLabels.push(`${pill.dataset.variantName}: ${pill.dataset.variantValue}`);
    const pr = parseFloat(pill.dataset.variantPrice);
    if (!isNaN(pr)) overridePrice = pr;
  });

  const variantLabel = variantLabels.join(', ');
  addToCart(productId, event, variantId, variantLabel, overridePrice);
  closeProductDetail();
}

function closeProductDetail() {
  const modal = document.getElementById('product-detail-modal');
  if (modal) modal.style.display = 'none';
}

window.startFlashSaleTimer = startFlashSaleTimer;
window.renderCategoryPills = renderCategoryPills;
window.filterCategory = filterCategory;
window.setupSearchListener = setupSearchListener;
window.renderCatalog = renderCatalog;
window.toggleWishlist = toggleWishlist;
window.updateWishlistButtons = updateWishlistButtons;
window.showProductDetail = showProductDetail;
window.closeProductDetail = closeProductDetail;
window.selectVariant = selectVariant;
window.addToCartWithVariant = addToCartWithVariant;
