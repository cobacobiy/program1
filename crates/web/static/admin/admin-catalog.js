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

// Window Exports
window.renderMasterProducts = renderMasterProducts;
window.openCreateProductModal = openCreateProductModal;
window.closeCreateProductModal = closeCreateProductModal;
window.uploadProductImage = uploadProductImage;
window.handleProductImageUpload = handleProductImageUpload;
window.handleCreateProductSubmit = handleCreateProductSubmit;
