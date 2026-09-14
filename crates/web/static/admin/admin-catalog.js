/* ==========================================================================
   ADMIN CATALOG & PRODUCT ENGINE
   File: /crates/web/static/admin/admin-catalog.js
   ========================================================================== */

function renderMasterProducts(catalog) {
  const tbody = document.getElementById("master-products-tbody");
  if (!tbody) return;

  tbody.innerHTML = catalog.map(p => `
    <tr>
      <td>
        <div class="product-cell">
          <img src="${escapeHtml(p.image_url)}" class="product-thumb" alt="Product" onerror="this.src='https://via.placeholder.com/40'">
          <div class="product-meta">
            <strong>${escapeHtml(p.name)}</strong>
            <span>${escapeHtml(p.description || "")}</span>
          </div>
        </div>
      </td>
      <td><strong style="font-family:'JetBrains Mono'; color:var(--cyan)">${escapeHtml(p.sku)}</strong></td>
      <td>${escapeHtml(p.category)}</td>
      <td style="font-weight:700; color:var(--emerald)">Rp ${p.price.toLocaleString("id-ID")}</td>
      <td><strong>${p.stock} units</strong></td>
      <td>
        <button class="btn-action-sm btn-action" onclick="openVariantModal('${p.id}', '${escapeHtml(p.name)}')">
          📦 Variants
        </button>
      </td>
    </tr>
  `).join("");
}

function openCreateProductModal() {
  const form = document.getElementById("create-product-form");
  if (form) form.reset();
  const urlEl = document.getElementById("product-image-url");
  if (urlEl) urlEl.value = "";
  const statusEl = document.getElementById("upload-file-status");
  if (statusEl) {
    statusEl.innerText = "Belum ada file dipilih";
    statusEl.style.color = "var(--text-muted)";
  }
  const previewBox = document.getElementById("product-image-preview-box");
  if (previewBox) previewBox.style.display = "none";
  const modal = document.getElementById("create-product-modal");
  if (modal) modal.style.display = "flex";
}

function closeCreateProductModal() {
  const modal = document.getElementById("create-product-modal");
  if (modal) modal.style.display = "none";
}

async function uploadProductImage(file) {
  const token = window.AdminState ? window.AdminState.authToken : window.authToken;
  const formData = new FormData();
  formData.append("image", file);

  const res = await fetch("/api/v1/uploads/images", {
    method: "POST",
    headers: {
      "Authorization": `Bearer ${token}`
    },
    body: formData
  });

  if (!res.ok) {
    const err = await res.json().catch(() => ({ message: "Gagal mengunggah gambar" }));
    throw new Error(err.message || err.error?.message || "Upload gagal");
  }

  return await res.json();
}

async function handleProductImageUpload(input) {
  const file = input.files && input.files[0];
  if (!file) return;

  const statusEl = document.getElementById("upload-file-status");
  const previewBox = document.getElementById("product-image-preview-box");
  const previewImg = document.getElementById("product-image-preview");
  const previewName = document.getElementById("upload-preview-name");
  const previewMeta = document.getElementById("upload-preview-meta");
  const urlInput = document.getElementById("product-image-url");
  const submitBtn = document.getElementById("create-prod-submit-btn");

  if (file.size > 5 * 1024 * 1024) {
    alert("Ukuran file melebihi batas 5 MB!");
    input.value = "";
    return;
  }

  try {
    statusEl.innerText = "Mengunggah gambar...";
    statusEl.style.color = "var(--cyan)";
    if (submitBtn) submitBtn.disabled = true;

    const data = await uploadProductImage(file);
    urlInput.value = data.url;
    statusEl.innerText = "✅ Berhasil diunggah!";
    statusEl.style.color = "var(--emerald)";

    previewImg.src = data.url;
    previewName.innerText = file.name;
    previewMeta.innerText = `${(data.size_bytes / 1024).toFixed(1)} KB • URL: ${data.url}`;
    previewBox.style.display = "flex";
  } catch (err) {
    console.error("Error uploading product image:", err);
    statusEl.innerText = `❌ ${err.message}`;
    statusEl.style.color = "var(--rose)";
    alert(`Upload gagal: ${err.message}`);
  } finally {
    if (submitBtn) submitBtn.disabled = false;
  }
}

async function handleCreateProductSubmit(event) {
  if (event) event.preventDefault();
  const sku = document.getElementById("create-prod-sku").value.trim();
  const name = document.getElementById("create-prod-name").value.trim();
  const category = document.getElementById("create-prod-category").value.trim();
  const price = parseInt(document.getElementById("create-prod-price").value, 10);
  const initialStock = parseInt(document.getElementById("create-prod-stock").value, 10);
  const description = document.getElementById("create-prod-desc").value.trim();
  const imageUrl = document.getElementById("product-image-url").value.trim();

  const payload = {
    sku,
    name,
    category,
    price,
    initial_stock: initialStock,
    description: description || null,
    image_url: imageUrl || "https://placehold.co/400"
  };

  try {
    const res = await authFetch("/api/v1/catalog", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload)
    });

    if (!res.ok) {
      const err = await res.json().catch(() => ({ message: "Gagal membuat produk" }));
      alert(`Error: ${err.message || err.error?.message || "Gagal membuat produk"}`);
      return;
    }

    closeCreateProductModal();
    alert(`Produk ${name} (${sku}) berhasil dibuat!`);
    await loadData();
  } catch (e) {
    console.error("Error submitting product:", e);
    alert(`Terjadi kesalahan: ${e.message}`);
  }
}

// --- PRODUCT VARIANT MANAGEMENT ENGINE ---
async function openVariantModal(productId, productName) {
  let modal = document.getElementById("variant-modal");
  if (!modal) {
    modal = document.createElement("div");
    modal.id = "variant-modal";
    modal.className = "modal-overlay";
    document.body.appendChild(modal);
  }

  modal.innerHTML = `
    <div class="modal-content" style="max-width:750px">
      <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:1rem">
        <h3 style="margin:0">📦 Kelola Varian — ${escapeHtml(productName)}</h3>
        <button class="btn-action-sm btn-action" onclick="closeVariantModal()">✕</button>
      </div>
      <div id="variant-list-container">
        <p style="color:var(--text-muted)">Memuat daftar varian...</p>
      </div>
      <hr style="border-color:rgba(255,255,255,0.1); margin:1.5rem 0">
      <h4 style="margin:0 0 0.8rem 0; color:var(--cyan)">➕ Tambah Varian Baru</h4>
      <form id="create-variant-form" onsubmit="handleCreateVariant(event, '${productId}', '${escapeHtml(productName)}')">
        <div style="display:grid; grid-template-columns:1fr 1fr; gap:0.75rem">
          <div>
            <label style="font-size:0.75rem; color:var(--text-muted)">Nama Varian (misal: Ukuran, Warna)*</label>
            <input id="var-name" class="input-field" placeholder="Ukuran / Warna" required style="width:100%; padding:0.4rem; background:rgba(255,255,255,0.05); border:1px solid rgba(255,255,255,0.1); color:#fff; border-radius:4px">
          </div>
          <div>
            <label style="font-size:0.75rem; color:var(--text-muted)">Nilai Varian (misal: XL, Merah)*</label>
            <input id="var-value" class="input-field" placeholder="XL / Merah" required style="width:100%; padding:0.4rem; background:rgba(255,255,255,0.05); border:1px solid rgba(255,255,255,0.1); color:#fff; border-radius:4px">
          </div>
          <div>
            <label style="font-size:0.75rem; color:var(--text-muted)">SKU Khusus Varian (Opsional)</label>
            <input id="var-sku" class="input-field" placeholder="SKU-VAR-001" style="width:100%; padding:0.4rem; background:rgba(255,255,255,0.05); border:1px solid rgba(255,255,255,0.1); color:#fff; border-radius:4px">
          </div>
          <div>
            <label style="font-size:0.75rem; color:var(--text-muted)">Harga Override Rp (Kosongkan jika sama)</label>
            <input id="var-price" type="number" class="input-field" placeholder="e.g. 150000" style="width:100%; padding:0.4rem; background:rgba(255,255,255,0.05); border:1px solid rgba(255,255,255,0.1); color:#fff; border-radius:4px">
          </div>
          <div>
            <label style="font-size:0.75rem; color:var(--text-muted)">Stok Awal Varian*</label>
            <input id="var-stock" type="number" min="0" value="0" class="input-field" required style="width:100%; padding:0.4rem; background:rgba(255,255,255,0.05); border:1px solid rgba(255,255,255,0.1); color:#fff; border-radius:4px">
          </div>
        </div>
        <div style="display:flex; justify-content:flex-end; gap:0.5rem; margin-top:1rem">
          <button type="button" class="btn-action-sm btn-action" onclick="closeVariantModal()">Batal</button>
          <button type="submit" class="btn-action btn-primary" id="btn-save-variant">💾 Simpan Varian</button>
        </div>
      </form>
    </div>
  `;
  modal.style.display = "flex";

  await loadAndRenderVariants(productId, productName);
}

async function loadAndRenderVariants(productId, productName) {
  const container = document.getElementById("variant-list-container");
  if (!container) return;

  try {
    const res = await fetch(`/api/v1/catalog/${productId}/variants`);
    const variants = res.ok ? await res.json() : [];
    container.innerHTML = renderVariantTable(variants, productId, productName);
  } catch (e) {
    container.innerHTML = `<p style="color:var(--rose)">Gagal memuat varian: ${escapeHtml(e.message)}</p>`;
  }
}

function renderVariantTable(variants, productId, productName) {
  if (!variants || variants.length === 0) {
    return '<p style="color:var(--text-muted); font-size:0.85rem; padding:1rem 0">Belum ada varian untuk produk ini. Tambahkan varian menggunakan formulir di bawah.</p>';
  }
  return `
    <table style="width:100%; margin-top:0.5rem; border-collapse:collapse; font-size:0.85rem">
      <thead>
        <tr style="border-bottom:1px solid rgba(255,255,255,0.1); text-align:left">
          <th style="padding:0.4rem">Nama Varian</th>
          <th style="padding:0.4rem">Nilai</th>
          <th style="padding:0.4rem">SKU</th>
          <th style="padding:0.4rem">Harga Override</th>
          <th style="padding:0.4rem">Stok</th>
          <th style="padding:0.4rem; text-align:center">Aksi</th>
        </tr>
      </thead>
      <tbody>
        ${variants.map(v => `
          <tr style="border-bottom:1px solid rgba(255,255,255,0.05)">
            <td style="padding:0.5rem 0.4rem"><strong>${escapeHtml(v.variant_name)}</strong></td>
            <td style="padding:0.5rem 0.4rem"><span style="background:rgba(255,255,255,0.1); padding:0.2rem 0.5rem; border-radius:4px">${escapeHtml(v.variant_value)}</span></td>
            <td style="padding:0.5rem 0.4rem; font-family:'JetBrains Mono'; color:var(--cyan)">${v.sku ? escapeHtml(v.sku) : "-"}</td>
            <td style="padding:0.5rem 0.4rem; color:var(--emerald)">${v.price_override ? 'Rp ' + v.price_override.toLocaleString('id-ID') : '<span style="color:var(--text-muted)">Default Produk</span>'}</td>
            <td style="padding:0.5rem 0.4rem"><strong>${v.stock_quantity} units</strong></td>
            <td style="padding:0.5rem 0.4rem; text-align:center">
              <button class="btn-action-sm btn-action" style="color:var(--rose); border-color:rgba(239,68,68,0.3)" onclick="deleteVariant('${productId}', '${v.id}', '${escapeHtml(productName)}')">
                🗑️ Hapus
              </button>
            </td>
          </tr>
        `).join("")}
      </tbody>
    </table>
  `;
}

function closeVariantModal() {
  const modal = document.getElementById("variant-modal");
  if (modal) modal.style.display = "none";
}

async function handleCreateVariant(event, productId, productName) {
  if (event) event.preventDefault();
  const token = window.AdminState ? window.AdminState.authToken : window.authToken;

  const priceVal = document.getElementById("var-price").value;
  const payload = {
    variant_name: document.getElementById("var-name").value.trim(),
    variant_value: document.getElementById("var-value").value.trim(),
    sku: document.getElementById("var-sku").value.trim() || null,
    price_override: priceVal ? parseFloat(priceVal) : null,
    stock_quantity: parseInt(document.getElementById("var-stock").value, 10) || 0
  };

  const btn = document.getElementById("btn-save-variant");
  if (btn) btn.disabled = true;

  try {
    const res = await fetch(`/api/v1/catalog/${productId}/variants`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Authorization": `Bearer ${token}`
      },
      body: JSON.stringify(payload)
    });

    if (!res.ok) {
      const err = await res.json().catch(() => ({}));
      alert(`Gagal menambah varian: ${err.message || err.error || "Terjadi kesalahan"}`);
      return;
    }

    alert("Varian berhasil ditambahkan!");
    document.getElementById("create-variant-form").reset();
    await loadAndRenderVariants(productId, productName);
  } catch (e) {
    alert(`Error: ${e.message}`);
  } finally {
    if (btn) btn.disabled = false;
  }
}

async function deleteVariant(productId, variantId, productName) {
  if (!confirm("Apakah Anda yakin ingin menghapus varian ini?")) return;
  const token = window.AdminState ? window.AdminState.authToken : window.authToken;

  try {
    const res = await fetch(`/api/v1/catalog/${productId}/variants/${variantId}`, {
      method: "DELETE",
      headers: {
        "Authorization": `Bearer ${token}`
      }
    });

    if (!res.ok) {
      const err = await res.json().catch(() => ({}));
      alert(`Gagal menghapus varian: ${err.message || "Error"}`);
      return;
    }

    await loadAndRenderVariants(productId, productName);
  } catch (e) {
    alert(`Error: ${e.message}`);
  }
}

// Window Exports
window.renderMasterProducts = renderMasterProducts;
window.openCreateProductModal = openCreateProductModal;
window.closeCreateProductModal = closeCreateProductModal;
window.uploadProductImage = uploadProductImage;
window.handleProductImageUpload = handleProductImageUpload;
window.handleCreateProductSubmit = handleCreateProductSubmit;
window.openVariantModal = openVariantModal;
window.closeVariantModal = closeVariantModal;
window.handleCreateVariant = handleCreateVariant;
window.deleteVariant = deleteVariant;

