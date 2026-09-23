<script lang="ts">
  import type { Category, CreateCategoryPayload, UpdateCategoryPayload } from '../types';
  import { toast } from '../toast.svelte';
  import { i18n } from '../i18n.svelte';
  import { fetchAdmin } from './adminApi';

  interface Props {
    categories: Category[];
    loading: boolean;
    onRefresh: () => Promise<void> | void;
  }

  let { categories, loading, onRefresh }: Props = $props();

  let isAddCategoryOpen = $state(false);
  let isEditCategoryOpen = $state(false);
  let selectedCategoryForEdit = $state<Category | null>(null);

  let newCatName = $state('');
  let newCatSlug = $state('');
  let newCatDesc = $state('');
  let newCatIcon = $state('📦');
  let newCatSortOrder = $state(0);
  let isCreatingCategory = $state(false);

  let editCatName = $state('');
  let editCatSlug = $state('');
  let editCatDesc = $state('');
  let editCatIcon = $state('📦');
  let editCatSortOrder = $state(0);
  let editCatIsActive = $state(true);
  let isUpdatingCategory = $state(false);

  async function handleCreateCategory() {
    if (!newCatName.trim()) {
      toast.error('Nama kategori harus diisi');
      return;
    }
    isCreatingCategory = true;
    try {
      const payload: CreateCategoryPayload = {
        name: newCatName.trim(),
        slug: newCatSlug.trim() || newCatName.trim().toLowerCase().replace(/\s+/g, '-'),
        description: newCatDesc.trim() || null,
        icon: newCatIcon.trim() || '📦',
        sort_order: newCatSortOrder,
      };
      await fetchAdmin('/api/v1/admin/categories', {
        method: 'POST',
        body: JSON.stringify(payload),
      });
      toast.success('Kategori berhasil dibuat!');
      isAddCategoryOpen = false;
      newCatName = '';
      newCatSlug = '';
      newCatDesc = '';
      newCatIcon = '📦';
      newCatSortOrder = 0;
      await onRefresh();
    } catch (err: any) {
      toast.error(err.message || 'Gagal membuat kategori');
    } finally {
      isCreatingCategory = false;
    }
  }

  function openEditCategory(cat: Category) {
    selectedCategoryForEdit = cat;
    editCatName = cat.name;
    editCatSlug = cat.slug;
    editCatDesc = cat.description || '';
    editCatIcon = cat.icon || '📦';
    editCatSortOrder = cat.sort_order;
    editCatIsActive = cat.is_active;
    isEditCategoryOpen = true;
  }

  async function handleUpdateCategory() {
    if (!selectedCategoryForEdit) return;
    if (!editCatName.trim()) {
      toast.error('Nama kategori harus diisi');
      return;
    }
    isUpdatingCategory = true;
    try {
      const payload: UpdateCategoryPayload = {
        name: editCatName.trim(),
        slug: editCatSlug.trim() || editCatName.trim().toLowerCase().replace(/\s+/g, '-'),
        description: editCatDesc.trim() || null,
        icon: editCatIcon.trim() || '📦',
        sort_order: editCatSortOrder,
        is_active: editCatIsActive,
      };
      await fetchAdmin(`/api/v1/admin/categories/${selectedCategoryForEdit.id}`, {
        method: 'PUT',
        body: JSON.stringify(payload),
      });
      toast.success('Kategori berhasil diperbarui!');
      isEditCategoryOpen = false;
      selectedCategoryForEdit = null;
      await onRefresh();
    } catch (err: any) {
      toast.error(err.message || 'Gagal memperbarui kategori');
    } finally {
      isUpdatingCategory = false;
    }
  }

  async function handleDeleteCategory(cat: Category) {
    if (cat.product_count > 0) {
      toast.error(`Kategori "${cat.name}" masih memiliki ${cat.product_count} produk terhubung.`);
      return;
    }
    if (!confirm(`Yakin ingin menghapus kategori "${cat.name}"?`)) return;
    try {
      await fetchAdmin(`/api/v1/admin/categories/${cat.id}`, {
        method: 'DELETE',
      });
      toast.success(`Kategori "${cat.name}" berhasil dihapus.`);
      await onRefresh();
    } catch (err: any) {
      toast.error(err.message || 'Gagal menghapus kategori');
    }
  }
</script>

<div class="section-panel">
  <div class="panel-top">
    <div>
      <h2>🏷️ {i18n.t('admin.category_mgmt', 'Manajemen Kategori Produk')}</h2>
      <span class="panel-subtitle">Kelola taksonomi, ikon emoji, urutan, dan pengelompokan produk toko</span>
    </div>
    <button class="btn-add-prod" onclick={() => isAddCategoryOpen = true}>+ {i18n.t('admin.add_category', 'Tambah Kategori Baru')}</button>
  </div>

  {#if loading}
    <p class="empty-text">Memuat kategori...</p>
  {:else if categories.length === 0}
    <p class="empty-text">Belum ada kategori yang ditambahkan.</p>
  {:else}
    <table class="table-custom">
      <thead>
        <tr>
          <th>Ikon</th>
          <th>Nama Kategori</th>
          <th>Slug URL</th>
          <th>Deskripsi</th>
          <th>Urutan</th>
          <th>Produk Terhubung</th>
          <th>Status</th>
          <th>Aksi</th>
        </tr>
      </thead>
      <tbody>
        {#each categories as cat (cat.id)}
          <tr>
            <td style="font-size: 1.4rem; text-align: center;">{cat.icon || '📦'}</td>
            <td><strong>{cat.name}</strong></td>
            <td><code>{cat.slug}</code></td>
            <td class="text-muted">{cat.description || '-'}</td>
            <td>{cat.sort_order}</td>
            <td>
              <span class="badge-tag">{cat.product_count} produk</span>
            </td>
            <td>
              <span class="status-pill {cat.is_active ? 'delivered' : 'cancelled'}">
                {cat.is_active ? 'Aktif' : 'Non-aktif'}
              </span>
            </td>
            <td>
              <div style="display: flex; gap: 0.4rem;">
                <button class="btn-action" onclick={() => openEditCategory(cat)}>
                  ✏️ Edit
                </button>
                <button
                  class="btn-action"
                  style="color: #ef4444;"
                  title={cat.product_count > 0 ? "Tidak dapat dihapus karena memiliki produk terhubung" : "Hapus kategori"}
                  disabled={cat.product_count > 0}
                  onclick={() => handleDeleteCategory(cat)}
                >
                  🗑️ Hapus
                </button>
              </div>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<!-- Modal Tambah Kategori -->
{#if isAddCategoryOpen}
  <div class="modal-overlay" onclick={() => isAddCategoryOpen = false} role="button" tabindex="0" onkeydown={(e) => e.key === 'Escape' && (isAddCategoryOpen = false)}>
    <div class="modal-card" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <div class="modal-header">
        <h3>🏷️ Tambah Kategori Baru</h3>
        <button class="close-btn" onclick={() => isAddCategoryOpen = false}>&times;</button>
      </div>

      <form onsubmit={(e) => { e.preventDefault(); handleCreateCategory(); }} class="prod-form">
        <div class="row-fields">
          <label style="flex: 1;">
            Ikon (Emoji):
            <input type="text" bind:value={newCatIcon} placeholder="📦" style="font-size: 1.2rem; text-align: center;" />
          </label>
          <label style="flex: 3;">
            Nama Kategori:
            <input type="text" bind:value={newCatName} required placeholder="Contoh: Gaming Accessories" />
          </label>
        </div>
        <label>
          Slug URL (Otomatis jika kosong):
          <input type="text" bind:value={newCatSlug} placeholder="gaming-accessories" />
        </label>
        <label>
          Deskripsi (Opsional):
          <textarea bind:value={newCatDesc} rows="2" placeholder="Penjelasan singkat kategori..."></textarea>
        </label>
        <label>
          Urutan Tampilan (Sort Order):
          <input type="number" bind:value={newCatSortOrder} min="0" />
        </label>

        <div class="modal-actions">
          <button type="button" class="btn-cancel" onclick={() => isAddCategoryOpen = false}>Batal</button>
          <button type="submit" class="btn-save" disabled={isCreatingCategory}>
            {isCreatingCategory ? 'Menyimpan...' : 'Simpan Kategori'}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Modal Edit Kategori -->
{#if isEditCategoryOpen && selectedCategoryForEdit}
  <div class="modal-overlay" onclick={() => isEditCategoryOpen = false} role="button" tabindex="0" onkeydown={(e) => e.key === 'Escape' && (isEditCategoryOpen = false)}>
    <div class="modal-card" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <div class="modal-header">
        <h3>✏️ Edit Kategori: {selectedCategoryForEdit.name}</h3>
        <button class="close-btn" onclick={() => isEditCategoryOpen = false}>&times;</button>
      </div>

      <form onsubmit={(e) => { e.preventDefault(); handleUpdateCategory(); }} class="prod-form">
        <div class="row-fields">
          <label style="flex: 1;">
            Ikon:
            <input type="text" bind:value={editCatIcon} style="font-size: 1.2rem; text-align: center;" />
          </label>
          <label style="flex: 3;">
            Nama Kategori:
            <input type="text" bind:value={editCatName} required />
          </label>
        </div>
        <label>
          Slug URL:
          <input type="text" bind:value={editCatSlug} required />
        </label>
        <label>
          Deskripsi:
          <textarea bind:value={editCatDesc} rows="2"></textarea>
        </label>
        <div class="row-fields">
          <label>
            Urutan:
            <input type="number" bind:value={editCatSortOrder} min="0" />
          </label>
          <label style="display: flex; flex-direction: row; align-items: center; gap: 0.5rem; margin-top: 1.2rem;">
            <input type="checkbox" bind:checked={editCatIsActive} style="width: 1.2rem; height: 1.2rem;" />
            <span>Status Aktif</span>
          </label>
        </div>

        <div class="modal-actions">
          <button type="button" class="btn-cancel" onclick={() => isEditCategoryOpen = false}>Batal</button>
          <button type="submit" class="btn-save" disabled={isUpdatingCategory}>
            {isUpdatingCategory ? 'Menyimpan...' : 'Perbarui Kategori'}
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
  .panel-subtitle { font-size: 0.85rem; color: #94a3b8; margin-top: 0.25rem; display: inline-block; }
  .btn-add-prod {
    background: #059669; color: #fff; border: none; padding: 0.55rem 1rem;
    border-radius: 6px; font-weight: bold; cursor: pointer;
  }
  .table-custom { width: 100%; border-collapse: collapse; margin-top: 1rem; }
  .table-custom th, .table-custom td {
    padding: 0.75rem 1rem; text-align: left; border-bottom: 1px solid #1e293b; font-size: 0.9rem;
  }
  .table-custom th { background: #1e293b; color: #94a3b8; font-size: 0.8rem; }
  .badge-tag { background: #334155; padding: 0.2rem 0.5rem; border-radius: 4px; font-size: 0.75rem; color: #cbd5e1; }
  .status-pill { padding: 0.2rem 0.5rem; border-radius: 4px; font-size: 0.75rem; font-weight: bold; }
  .status-pill.delivered { background: #065f46; color: #a7f3d0; }
  .status-pill.cancelled { background: #7f1d1d; color: #fecaca; }
  .btn-action {
    background: #334155; color: #fff; border: none; padding: 0.35rem 0.7rem;
    border-radius: 4px; cursor: pointer; font-size: 0.8rem;
  }
  .btn-action:disabled { opacity: 0.4; cursor: not-allowed; }
  .text-muted { color: #64748b; font-size: 0.85rem; }
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
  .modal-actions { display: flex; justify-content: flex-end; gap: 0.75rem; margin-top: 0.75rem; }
  .btn-cancel { background: #334155; color: #fff; border: none; padding: 0.6rem 1.2rem; border-radius: 6px; cursor: pointer; }
  .btn-save { background: #059669; color: #fff; border: none; padding: 0.6rem 1.2rem; border-radius: 6px; font-weight: bold; cursor: pointer; }
</style>
