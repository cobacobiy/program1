<script lang="ts">
  import type { Product, ProductVariant, CreateVariantPayload, Category } from '../types';
  import { formatRupiah } from '../currency';
  import { toast } from '../toast.svelte';
  import { i18n } from '../i18n.svelte';
  import { adminAuth } from '../adminAuth.svelte';
  import { fetchAdmin } from './adminApi';

  interface Props {
    products: Product[];
    categories: Category[];
    loading: boolean;
    onRefresh: () => Promise<void> | void;
  }

  let { products, categories, loading, onRefresh }: Props = $props();

  // New product modal state
  let isAddProductOpen = $state(false);
  let newName = $state('');
  let newDesc = $state('');
  let newPrice = $state(50000);
  let newStock = $state(20);
  let newCategory = $state('Umum');
  let newCategoryId = $state<string>('');
  let selectedFile = $state<File | null>(null);
  let isUploading = $state(false);

  // Variant Modal State
  let selectedProductForVariants = $state<Product | null>(null);
  let variants = $state<ProductVariant[]>([]);
  let variantsLoading = $state(false);
  let varName = $state('Ukuran');
  let varValue = $state('');
  let varSku = $state('');
  let varPriceOverride = $state<number | null>(null);
  let varStock = $state(10);
  let varSaving = $state(false);

  async function openVariantModal(prod: Product) {
    selectedProductForVariants = prod;
    varValue = '';
    varSku = '';
    varPriceOverride = null;
    await fetchVariants(prod.id);
  }

  async function fetchVariants(productId: string) {
    variantsLoading = true;
    try {
      variants = await fetchAdmin<ProductVariant[]>(`/api/v1/catalog/${productId}/variants`);
    } catch (err: any) {
      toast.error(err.message || 'Gagal mengambil daftar varian');
    } finally {
      variantsLoading = false;
    }
  }

  async function handleCreateProduct(e: Event) {
    e.preventDefault();
    isUploading = true;

    try {
      let imageUrl: string | null = null;

      if (selectedFile) {
        const form = new FormData();
        form.append('image', selectedFile);
        const upRes = await fetch('/v1/upload', {
          method: 'POST',
          headers: { Authorization: `Bearer ${adminAuth.token}` },
          body: form,
        });
        if (upRes.ok) {
          const upData = await upRes.json();
          imageUrl = upData.url;
        }
      }

      await fetchAdmin('/api/v1/catalog', {
        method: 'POST',
        body: JSON.stringify({
          name: newName,
          description: newDesc,
          price_cents: newPrice,
          stock: newStock,
          category: newCategory,
          category_id: newCategoryId || null,
          image_url: imageUrl,
        }),
      }).catch(async () => {
        return fetchAdmin('/catalog', {
          method: 'POST',
          body: JSON.stringify({
            name: newName,
            description: newDesc,
            price_cents: newPrice,
            stock: newStock,
            category: newCategory,
            category_id: newCategoryId || null,
            image_url: imageUrl,
          }),
        });
      });

      toast.success(`Produk "${newName}" berhasil ditambahkan!`);
      isAddProductOpen = false;
      newName = '';
      newDesc = '';
      selectedFile = null;
      await onRefresh();
    } catch (err: any) {
      toast.error(err.message || 'Gagal membuat produk.');
    } finally {
      isUploading = false;
    }
  }

  async function handleAddVariant() {
    if (!selectedProductForVariants) return;
    if (!varName.trim() || !varValue.trim()) {
      toast.error('Nama dan Nilai varian wajib diisi!');
      return;
    }

    varSaving = true;
    try {
      const payload: CreateVariantPayload = {
        variant_name: varName.trim(),
        variant_value: varValue.trim(),
        sku: varSku.trim() || null,
        price_override: varPriceOverride && varPriceOverride > 0 ? varPriceOverride : null,
        stock_quantity: Number(varStock) || 0,
      };

      await fetchAdmin(`/api/v1/catalog/${selectedProductForVariants.id}/variants`, {
        method: 'POST',
        body: JSON.stringify(payload),
      });

      toast.success('Varian berhasil ditambahkan!');
      varValue = '';
      varSku = '';
      varPriceOverride = null;
      await fetchVariants(selectedProductForVariants.id);
    } catch (err: any) {
      toast.error(err.message || 'Gagal menambahkan varian');
    } finally {
      varSaving = false;
    }
  }

  async function handleDeleteVariant(variantId: string) {
    if (!selectedProductForVariants) return;
    if (!confirm('Hapus varian ini?')) return;

    try {
      await fetchAdmin(`/api/v1/catalog/${selectedProductForVariants.id}/variants/${variantId}`, {
        method: 'DELETE',
      });
      toast.success('Varian berhasil dihapus!');
      await fetchVariants(selectedProductForVariants.id);
    } catch (err: any) {
      toast.error(err.message || 'Gagal menghapus varian');
    }
  }
</script>

<div class="section-panel">
  <div class="panel-top">
    <h2>📦 {i18n.t('admin.catalog_mgmt', 'Manajemen Katalog Produk')}</h2>
    <button class="btn-add-prod" onclick={() => isAddProductOpen = true}>+ {i18n.t('admin.add_product', 'Tambah Produk Baru')}</button>
  </div>

  {#if loading}
    <p class="empty-text">Memuat katalog...</p>
  {:else}
    <table class="table-custom">
      <thead>
        <tr>
          <th>Gambar</th>
          <th>Nama Produk</th>
          <th>Kategori</th>
          <th>Harga</th>
          <th>Stok</th>
          <th>Varian</th>
        </tr>
      </thead>
      <tbody>
        {#each products as p (p.id)}
          <tr>
            <td class="td-img">
              {#if p.image_url}
                <img src={p.image_url} alt={p.name} />
              {:else}
                <span>📦</span>
              {/if}
            </td>
            <td><strong>{p.name}</strong></td>
            <td><span class="badge-tag">{p.category || 'Umum'}</span></td>
            <td>{formatRupiah(p.price_cents)}</td>
            <td><span class="badge-stock" class:empty={p.stock <= 0}>{p.stock} unit</span></td>
            <td>
              <button class="btn-action primary" onclick={() => openVariantModal(p)}>
                ⚙️ Kelola Varian
              </button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<!-- Modal Tambah Produk -->
{#if isAddProductOpen}
  <div class="modal-overlay" onclick={() => isAddProductOpen = false} role="button" tabindex="0" onkeydown={(e) => e.key === 'Escape' && (isAddProductOpen = false)}>
    <div class="modal-card" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <div class="modal-header">
        <h3>📦 Tambah Produk ke Katalog</h3>
        <button class="close-btn" onclick={() => isAddProductOpen = false}>&times;</button>
      </div>

      <form onsubmit={handleCreateProduct} class="prod-form">
        <label>
          Nama Produk:
          <input type="text" bind:value={newName} required placeholder="Contoh: Kemeja Flanel Premium" />
        </label>
        <label>
          Deskripsi:
          <textarea bind:value={newDesc} rows="2" placeholder="Detail spesifikasi produk"></textarea>
        </label>
        <div class="row-fields">
          <label>
            Harga (Rp):
            <input type="number" bind:value={newPrice} min="1000" step="1000" required />
          </label>
          <label>
            Stok Awal:
            <input type="number" bind:value={newStock} min="1" required />
          </label>
        </div>
        <label>
          Pilih Kategori:
          <select
            class="select-custom"
            bind:value={newCategoryId}
            onchange={() => {
              const found = categories.find(c => c.id === newCategoryId);
              if (found) newCategory = found.name;
            }}
          >
            <option value="">-- Pilih dari Kategori Terdaftar --</option>
            {#each categories as cat (cat.id)}
              <option value={cat.id}>{cat.icon || '📦'} {cat.name}</option>
            {/each}
          </select>
        </label>
        <label>
          Atau Ketik Nama Kategori Manual:
          <input type="text" bind:value={newCategory} placeholder="Pakaian, Elektronik, Makanan, dll." />
        </label>
        <label>
          Cover Gambar Produk (Opsional):
          <input
            type="file"
            accept="image/*"
            onchange={(e: any) => selectedFile = e.target.files?.[0] || null}
          />
        </label>

        <div class="modal-actions">
          <button type="button" class="btn-cancel" onclick={() => isAddProductOpen = false}>Batal</button>
          <button type="submit" class="btn-save" disabled={isUploading}>
            {isUploading ? 'Menyimpan & Upload...' : 'Simpan Produk'}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Modal Kelola Varian Produk -->
{#if selectedProductForVariants}
  <div class="modal-overlay" onclick={() => selectedProductForVariants = null} role="presentation">
    <div class="modal-card modal-variant-card" onclick={(e) => e.stopPropagation()} role="dialog">
      <div class="modal-header">
        <div>
          <h3>⚙️ Kelola Varian Produk</h3>
          <p class="var-prod-subtitle">{selectedProductForVariants.name} ({formatRupiah(selectedProductForVariants.price_cents)})</p>
        </div>
        <button class="close-btn" onclick={() => selectedProductForVariants = null}>&times;</button>
      </div>

      <div class="var-list-section">
        <h4>Daftar Varian Aktif</h4>
        {#if variantsLoading}
          <p class="empty-text">Memuat varian...</p>
        {:else if variants.length === 0}
          <p class="empty-text">Belum ada varian untuk produk ini. Tambahkan di bawah.</p>
        {:else}
          <table class="table-custom var-table">
            <thead>
              <tr>
                <th>Varian</th>
                <th>Nilai</th>
                <th>SKU</th>
                <th>Harga Override</th>
                <th>Stok</th>
                <th>Aksi</th>
              </tr>
            </thead>
            <tbody>
              {#each variants as v (v.id)}
                <tr>
                  <td><strong>{v.variant_name}</strong></td>
                  <td><span class="badge-tag">{v.variant_value}</span></td>
                  <td><code>{v.sku || '-'}</code></td>
                  <td>{v.price_override ? formatRupiah(v.price_override) : '(Standar)'}</td>
                  <td>{v.stock_quantity} unit</td>
                  <td>
                    <button class="btn-var-del" onclick={() => handleDeleteVariant(v.id)}>Hapus</button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>

      <form class="var-add-form" onsubmit={(e) => { e.preventDefault(); handleAddVariant(); }}>
        <h4>+ Tambah Varian Baru</h4>
        <div class="row-fields">
          <label>
            Nama Varian
            <input type="text" bind:value={varName} placeholder="Ukuran, Warna, dll" required />
          </label>
          <label>
            Nilai Varian
            <input type="text" bind:value={varValue} placeholder="XL, Merah, 256GB" required />
          </label>
        </div>
        <div class="row-fields">
          <label>
            SKU Varian (Opsional)
            <input type="text" bind:value={varSku} placeholder="SKU-PROD-VAR" />
          </label>
          <label>
            Harga Khusus (Rp)
            <input type="number" bind:value={varPriceOverride} placeholder="Kosongkan jika harga standar" min="0" />
          </label>
          <label>
            Stok Varian
            <input type="number" bind:value={varStock} min="0" required />
          </label>
        </div>
        <div class="modal-actions">
          <button type="button" class="btn-cancel" onclick={() => selectedProductForVariants = null}>Tutup</button>
          <button type="submit" class="btn-save" disabled={varSaving}>
            {varSaving ? 'Menyimpan...' : '+ Tambah Varian'}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .section-panel { width: 100%; }
  h2 { margin: 0; font-size: 1.25rem; color: #f8fafc; }
  .panel-top { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
  .btn-add-prod {
    background: #059669; color: #fff; border: none; padding: 0.55rem 1rem;
    border-radius: 6px; font-weight: bold; cursor: pointer;
  }
  .table-custom { width: 100%; border-collapse: collapse; margin-top: 1rem; }
  .table-custom th, .table-custom td {
    padding: 0.75rem 1rem; text-align: left; border-bottom: 1px solid #1e293b; font-size: 0.9rem;
  }
  .table-custom th { background: #1e293b; color: #94a3b8; font-size: 0.8rem; }
  .td-img { width: 44px; height: 44px; text-align: center; }
  .td-img img { width: 40px; height: 40px; object-fit: cover; border-radius: 4px; }
  .badge-tag { background: #334155; padding: 0.2rem 0.5rem; border-radius: 4px; font-size: 0.75rem; color: #cbd5e1; }
  .badge-stock { color: #10b981; font-weight: bold; }
  .badge-stock.empty { color: #ef4444; }
  .btn-action {
    background: #334155; color: #fff; border: none; padding: 0.35rem 0.7rem;
    border-radius: 4px; cursor: pointer; font-size: 0.8rem;
  }
  .btn-action.primary { background: #0284c7; font-weight: bold; }
  .empty-text { color: #94a3b8; text-align: center; padding: 2rem 0; }

  /* Modal */
  .modal-overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.7);
    display: flex; align-items: center; justify-content: center; z-index: 1000;
  }
  .modal-card {
    background: #0f172a; border: 1px solid #334155; border-radius: 12px;
    width: 90%; max-width: 480px; padding: 1.5rem; color: #f8fafc;
  }
  .modal-header { display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid #334155; padding-bottom: 0.5rem; }
  .modal-header h3 { margin: 0; font-size: 1.15rem; color: #f8fafc; }
  .close-btn { background: none; border: none; font-size: 1.4rem; color: #94a3b8; cursor: pointer; }
  .prod-form { display: flex; flex-direction: column; gap: 0.85rem; margin-top: 1rem; }
  .prod-form label { font-size: 0.85rem; color: #cbd5e1; display: flex; flex-direction: column; gap: 0.25rem; }
  .prod-form input, .prod-form textarea {
    background: #1e293b; border: 1px solid #334155; border-radius: 6px;
    padding: 0.6rem; color: #fff; font-family: inherit;
  }
  .row-fields { display: flex; gap: 1rem; }
  .row-fields label { flex: 1; }
  .select-custom {
    background: #1e293b; border: 1px solid #334155; border-radius: 6px;
    padding: 0.6rem; color: #fff; font-family: inherit;
  }
  .modal-actions { display: flex; justify-content: flex-end; gap: 0.75rem; margin-top: 0.75rem; }
  .btn-cancel { background: #334155; color: #fff; border: none; padding: 0.6rem 1.2rem; border-radius: 6px; cursor: pointer; }
  .btn-save { background: #059669; color: #fff; border: none; padding: 0.6rem 1.2rem; border-radius: 6px; font-weight: bold; cursor: pointer; }

  /* Variant Modal */
  .modal-variant-card { max-width: 680px; }
  .var-prod-subtitle { font-size: 0.85rem; color: #38bdf8; margin: 0.25rem 0 0; }
  .var-list-section { margin-top: 1rem; max-height: 220px; overflow-y: auto; }
  .var-list-section h4 { margin: 0 0 0.5rem; font-size: 0.95rem; color: #94a3b8; }
  .var-table th, .var-table td { padding: 0.5rem 0.75rem; font-size: 0.85rem; }
  .btn-var-del { background: #7f1d1d; color: #fecaca; border: none; padding: 0.25rem 0.5rem; border-radius: 4px; cursor: pointer; font-size: 0.75rem; }
  .btn-var-del:hover { background: #991b1b; }
  .var-add-form { margin-top: 1.5rem; border-top: 1px solid #334155; padding-top: 1rem; display: flex; flex-direction: column; gap: 0.75rem; }
  .var-add-form h4 { margin: 0 0 0.25rem; font-size: 0.95rem; color: #38bdf8; }
  .var-add-form label { font-size: 0.8rem; color: #cbd5e1; display: flex; flex-direction: column; gap: 0.2rem; }
  .var-add-form input { background: #1e293b; border: 1px solid #334155; border-radius: 6px; padding: 0.55rem; color: #fff; }
</style>
