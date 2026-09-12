/* ==========================================================================
   STOREFRONT PRODUCT CATALOG & SEARCH ENGINE
   File: /crates/web/static/store/store-catalog.js
   ========================================================================== */

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

// Render Products Grid (Shopee-Style Card)
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
          <button class="btn-add-cart" onclick="addToCart('${p.id}', event)">+ Beli</button>
        </div>
      </div>
    </div>
  `).join('');
}

window.startFlashSaleTimer = startFlashSaleTimer;
window.renderCategoryPills = renderCategoryPills;
window.filterCategory = filterCategory;
window.setupSearchListener = setupSearchListener;
window.renderCatalog = renderCatalog;
