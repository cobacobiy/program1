<script lang="ts">
  import { onMount } from 'svelte';
  import { adminAuth } from './adminAuth.svelte';
  import { toast } from './toast.svelte';
  import { formatRupiah } from './currency';
  import { i18n } from './i18n.svelte';
  import NotificationBell from './NotificationBell.svelte';
  import type { Product, ProductVariant, CreateVariantPayload, Coupon, CreateCouponPayload, DiscountType, ProductReview, PaginatedReviews, SalesReportResponse, Category, CreateCategoryPayload, UpdateCategoryPayload, ReturnRequest, ProcessReturnPayload, UpdateReturnStatusPayload } from './types';

  function handleAdminNotificationClick(item: any) {
    if (item.notification_type === 'return' || item.reference_type === 'return') {
      subTab = 'returns';
      loadAdminReturns();
    } else if (item.notification_type === 'new_order' || item.reference_type === 'order') {
      subTab = 'orders';
    } else if (item.notification_type === 'low_stock' || item.reference_type === 'product') {
      subTab = 'inventory';
    }
  }

  interface AdminOrder {
    id: string;
    total_amount_cents: number;
    status: string;
    created_at: string;
    tracking_number?: string | null;
  }

  interface InventoryRecord {
    id: string;
    product_name?: string;
    stock: number;
    safety_stock?: number;
  }

  interface AuditLogRecord {
    id: string;
    actor_username?: string;
    action: string;
    resource_type: string;
    created_at: string;
  }

  // Active sub-tab
  let subTab = $state<'kpi' | 'reports' | 'catalog' | 'categories' | 'inventory' | 'orders' | 'audit' | 'coupons' | 'reviews' | 'returns' | 'backup'>('kpi');

  // Database Backup state
  interface BackupFile {
    filename: string;
    size_bytes: number;
    created_at: string;
  }

  interface DatabaseHealth {
    status: string;
    engine: string;
    integrity: string;
  }

  let backups = $state<BackupFile[]>([]);
  let dbHealth = $state<DatabaseHealth | null>(null);
  let backupLoading = $state(false);
  let creatingBackup = $state(false);

  // Sales report state
  let reportDateFrom = $state('');
  let reportDateTo = $state('');
  let reportStatusFilter = $state('all');
  let reportLoading = $state(false);
  let salesReportData = $state<SalesReportResponse | null>(null);

  // Login form state
  let loginUser = $state('admin');
  let loginPass = $state('admin123');

  // Data states
  let products = $state<Product[]>([]);
  let orders = $state<AdminOrder[]>([]);
  let inventory = $state<InventoryRecord[]>([]);
  let auditLogs = $state<AuditLogRecord[]>([]);
  let coupons = $state<Coupon[]>([]);
  let adminReviews = $state<ProductReview[]>([]);
  let reviewsFilter = $state<'all' | 'visible' | 'hidden'>('all');
  let adminReviewsPage = $state(1);
  let adminReviewsPageSize = $state(20);
  let adminReviewsTotal = $state(0);
  let adminReviewsTotalPages = $state(1);
  let reviewsLoading = $state(false);
  let loading = $state(false);

  // New coupon modal state
  let isAddCouponOpen = $state(false);
  let newCouponCode = $state('');
  let newCouponDiscountType = $state<DiscountType>('PERCENTAGE');
  let newCouponDiscountVal = $state(10);
  let newCouponMinOrder = $state(0);
  let newCouponMaxCap = $state<number | null>(null);
  let newCouponLimit = $state<number | null>(null);
  let newCouponStartDate = $state(new Date().toISOString().slice(0, 16));
  let newCouponEndDate = $state(new Date(Date.now() + 30 * 86400000).toISOString().slice(0, 16));
  let isCreatingCoupon = $state(false);

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

  // Category management state
  let categories = $state<Category[]>([]);
  let categoriesLoading = $state(false);
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

  async function fetchCategories() {
    categoriesLoading = true;
    try {
      categories = await fetchAdmin<Category[]>('/api/v1/categories');
    } catch (err: any) {
      toast.error(err.message || 'Gagal mengambil kategori');
    } finally {
      categoriesLoading = false;
    }
  }

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
      await fetchCategories();
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
      await fetchCategories();
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
      await fetchCategories();
    } catch (err: any) {
      toast.error(err.message || 'Gagal menghapus kategori');
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

  async function handleToggleCoupon(couponId: string) {
    try {
      const updated = await fetchAdmin<Coupon>(`/api/v1/admin/coupons/${couponId}/toggle`, {
        method: 'PUT',
      });
      coupons = coupons.map(c => c.id === couponId ? updated : c);
      toast.success(`Status kupon ${updated.code} diubah!`);
    } catch (err: any) {
      toast.error(err.message || 'Gagal mengubah status kupon');
    }
  }

  async function handleDeleteCoupon(couponId: string, code: string) {
    if (!confirm(`Hapus kupon "${code}"? Tindakan ini tidak dapat dibatalkan.`)) return;
    try {
      await fetchAdmin(`/api/v1/admin/coupons/${couponId}`, {
        method: 'DELETE',
      });
      coupons = coupons.filter(c => c.id !== couponId);
      toast.success(`Kupon "${code}" berhasil dihapus.`);
    } catch (err: any) {
      toast.error(err.message || 'Gagal menghapus kupon');
    }
  }

  async function handleCreateCoupon(e: Event) {
    e.preventDefault();
    if (!newCouponCode.trim()) {
      toast.error('Kode kupon wajib diisi!');
      return;
    }

    isCreatingCoupon = true;
    try {
      const payload: CreateCouponPayload = {
        code: newCouponCode.trim().toUpperCase(),
        discount_type: newCouponDiscountType,
        discount_value: Number(newCouponDiscountVal),
        min_order_amount: newCouponMinOrder ? Number(newCouponMinOrder) : 0,
        max_discount_amount: newCouponMaxCap && newCouponMaxCap > 0 ? Number(newCouponMaxCap) : undefined,
        usage_limit: newCouponLimit && newCouponLimit > 0 ? Number(newCouponLimit) : undefined,
        start_date: new Date(newCouponStartDate).toISOString(),
        end_date: new Date(newCouponEndDate).toISOString(),
      };

      const created = await fetchAdmin<Coupon>('/api/v1/admin/coupons', {
        method: 'POST',
        body: JSON.stringify(payload),
      });

      coupons = [created, ...coupons];
      toast.success(`Kupon "${created.code}" berhasil dibuat!`);
      isAddCouponOpen = false;
      newCouponCode = '';
      newCouponDiscountVal = 10;
      newCouponMaxCap = null;
      newCouponLimit = null;
    } catch (err: any) {
      toast.error(err.message || 'Gagal membuat kupon.');
    } finally {
      isCreatingCoupon = false;
    }
  }

  async function fetchAdmin<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
    const headers = new Headers(options.headers || {});
    if (adminAuth.token) {
      headers.set('Authorization', `Bearer ${adminAuth.token}`);
    }
    if (!headers.has('Content-Type') && !(options.body instanceof FormData)) {
      headers.set('Content-Type', 'application/json');
    }

    const res = await fetch(endpoint, { ...options, headers });
    if (res.status === 401) {
      adminAuth.logout();
      throw new Error('Sesi admin berakhir.');
    }
    const data = await res.json().catch(() => ({}));
    if (!res.ok) {
      throw new Error(data.message || `Request failed (${res.status})`);
    }
    return data as T;
  }

  async function loadData() {
    if (!adminAuth.token) return;
    loading = true;
    try {
      if (subTab === 'kpi' || subTab === 'catalog' || subTab === 'categories') {
        const cat = await fetchAdmin<{ items: Product[] }>('/catalog?page=1&page_size=50').catch(() => ({ items: [] }));
        products = cat.items || [];
        await fetchCategories();
      }
      if (subTab === 'kpi' || subTab === 'orders') {
        const ord = await fetchAdmin<AdminOrder[]>('/orders').catch(() => []);
        orders = ord || [];
      }
      if (subTab === 'inventory') {
        const inv = await fetchAdmin<InventoryRecord[]>('/inventory').catch(() => []);
        inventory = inv || [];
      }
      if (subTab === 'audit') {
        const aud = await fetchAdmin<AuditLogRecord[]>('/audit/logs').catch(() => []);
        auditLogs = aud || [];
      }
      if (subTab === 'kpi' || subTab === 'coupons') {
        const coup = await fetchAdmin<Coupon[]>('/api/v1/admin/coupons').catch(() => []);
        coupons = coup || [];
      }
      if (subTab === 'kpi' || subTab === 'reviews') {
        await loadAdminReviews(adminReviewsPage);
      }
      if (subTab === 'kpi' || subTab === 'reports') {
        await loadSalesReport();
      }
      if (subTab === 'kpi' || subTab === 'returns') {
        await loadAdminReturns();
      }
      if (subTab === 'backup') {
        await loadBackupsAndHealth();
      }
    } catch (err: any) {
      toast.error(err.message || 'Gagal memuat data admin');
    } finally {
      loading = false;
    }
  }

  async function loadBackupsAndHealth() {
    backupLoading = true;
    try {
      const [bList, health] = await Promise.all([
        fetchAdmin<BackupFile[]>('/api/v1/admin/database/backups').catch(() => []),
        fetchAdmin<DatabaseHealth>('/api/v1/admin/database/health').catch(() => null)
      ]);
      backups = bList || [];
      dbHealth = health || null;
    } catch (err: any) {
      toast.error(err.message || 'Gagal memuat status database & backup');
    } finally {
      backupLoading = false;
    }
  }

  async function handleCreateBackup() {
    creatingBackup = true;
    try {
      const res = await fetchAdmin<BackupFile>('/api/v1/admin/database/backup', { method: 'POST' });
      toast.success(`Backup berhasil dibuat: ${res.filename}`);
      await loadBackupsAndHealth();
    } catch (err: any) {
      toast.error(err.message || 'Gagal membuat backup');
    } finally {
      creatingBackup = false;
    }
  }

  function downloadBackup(filename: string) {
    if (!adminAuth.token) return;
    const url = `/api/v1/admin/database/backups/${encodeURIComponent(filename)}/download`;
    fetch(url, {
      headers: {
        'Authorization': `Bearer ${adminAuth.token}`
      }
    })
    .then(async (res) => {
      if (!res.ok) throw new Error('Gagal mendownload backup');
      const blob = await res.blob();
      const blobUrl = window.URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = blobUrl;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      a.remove();
      window.URL.revokeObjectURL(blobUrl);
    })
    .catch((err) => {
      toast.error(err.message || 'Gagal mendownload backup');
    });
  }

  let adminReturns = $state<ReturnRequest[]>([]);
  let adminReturnsLoading = $state(false);
  let returnStatusFilter = $state<string>('');
  let rejectModalOpen = $state(false);
  let rejectTargetId = $state('');
  let rejectNotes = $state('');
  let isProcessingReturn = $state(false);

  async function loadAdminReturns() {
    adminReturnsLoading = true;
    try {
      const url = returnStatusFilter
        ? `/api/v1/admin/returns?status=${encodeURIComponent(returnStatusFilter)}`
        : '/api/v1/admin/returns';
      const res = await fetchAdmin<ReturnRequest[]>(url);
      adminReturns = res || [];
    } catch (err: any) {
      toast.error(err.message || 'Gagal memuat daftar retur.');
    } finally {
      adminReturnsLoading = false;
    }
  }

  function openRejectModal(id: string) {
    rejectTargetId = id;
    rejectNotes = '';
    rejectModalOpen = true;
  }

  async function processReturn(id: string, action: 'approve' | 'reject', notes?: string, override?: number) {
    isProcessingReturn = true;
    try {
      const payload: ProcessReturnPayload = {
        action,
        admin_notes: notes,
        refund_amount_override: override,
      };
      await fetchAdmin<ReturnRequest>(`/api/v1/admin/returns/${id}/process`, {
        method: 'POST',
        body: JSON.stringify(payload),
      });
      toast.success(action === 'approve' ? 'Pengajuan retur disetujui!' : 'Pengajuan retur ditolak.');
      rejectModalOpen = false;
      await loadAdminReturns();
    } catch (err: any) {
      toast.error(err.message || 'Gagal memproses retur.');
    } finally {
      isProcessingReturn = false;
    }
  }

  async function updateReturnStatus(id: string, status: 'return_shipped' | 'received' | 'refunded') {
    try {
      const payload: UpdateReturnStatusPayload = { status };
      await fetchAdmin<ReturnRequest>(`/api/v1/admin/returns/${id}/status`, {
        method: 'POST',
        body: JSON.stringify(payload),
      });
      toast.success(`Status retur diperbarui menjadi ${status.toUpperCase()}!`);
      await loadAdminReturns();
    } catch (err: any) {
      toast.error(err.message || 'Gagal memperbarui status retur.');
    }
  }

  async function loadAdminReviews(page = 1) {
    reviewsLoading = true;
    try {
      let url = `/api/v1/admin/reviews?page=${page}&page_size=${adminReviewsPageSize}`;
      if (reviewsFilter !== 'all') {
        url += `&status=${reviewsFilter}`;
      }
      const revs = await fetchAdmin<PaginatedReviews>(url);
      adminReviews = revs.items || [];
      adminReviewsTotal = revs.total;
      adminReviewsPage = revs.page;
      adminReviewsTotalPages = revs.total_pages;
    } catch (err: any) {
      toast.error(err.message || 'Gagal memuat ulasan');
    } finally {
      reviewsLoading = false;
    }
  }

  async function loadSalesReport() {
    reportLoading = true;
    try {
      const params = new URLSearchParams();
      if (reportDateFrom) params.set('date_from', reportDateFrom);
      if (reportDateTo) params.set('date_to', reportDateTo);
      if (reportStatusFilter && reportStatusFilter !== 'all') {
        params.set('status_filter', reportStatusFilter);
      }
      const qs = params.toString();
      const endpoint = qs ? `/api/v1/analytics/report?${qs}` : '/api/v1/analytics/report';
      salesReportData = await fetchAdmin<SalesReportResponse>(endpoint);
    } catch (err: any) {
      toast.error(err.message || 'Gagal memuat laporan penjualan');
    } finally {
      reportLoading = false;
    }
  }

  function exportCSV() {
    const rows = salesReportData?.rows || [];
    if (!rows.length) {
      toast.error('Tidak ada data transaksi untuk diekspor.');
      return;
    }

    let csv = 'Tanggal,Order ID,Buyer,Email,Produk,Qty,Subtotal (Rp),Ongkir (Rp),Diskon (Rp),Total (Rp),Status,Status Pembayaran\n';

    for (const row of rows) {
      const itemsStr = row.items.map(i => `${i.product_name} x${i.quantity}`).join('; ');
      const totalQty = row.items.reduce((s, i) => s + i.quantity, 0);
      const subtotalRp = row.subtotal_cents / 100;
      const shippingRp = row.shipping_cents / 100;
      const discountRp = row.discount_cents / 100;
      const totalRp = row.total_cents / 100;
      const emailStr = row.buyer_email || '-';

      csv += `"${row.order_date}","${row.order_id}","${row.buyer_name}","${emailStr}","${itemsStr.replace(/"/g, '""')}","${totalQty}","${subtotalRp}","${shippingRp}","${discountRp}","${totalRp}","${row.status}","${row.payment_status}"\n`;
    }

    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const link = document.createElement('a');
    link.href = URL.createObjectURL(blob);
    const dateStr = new Date().toISOString().slice(0, 10);
    link.download = `laporan-penjualan-${dateStr}.csv`;
    link.click();
    toast.success('Laporan penjualan CSV berhasil diunduh.');
  }

  function setReviewsFilter(filter: 'all' | 'visible' | 'hidden') {
    reviewsFilter = filter;
    adminReviewsPage = 1;
    loadAdminReviews(1);
  }

  function changeAdminReviewsPage(newPage: number) {
    if (newPage < 1 || newPage > adminReviewsTotalPages) return;
    loadAdminReviews(newPage);
  }

  async function handleToggleReviewVisibility(review: ProductReview) {
    const newVisibility = !review.is_visible;
    try {
      const updated = await fetchAdmin<ProductReview>(`/api/v1/admin/reviews/${review.id}/visibility`, {
        method: 'PATCH',
        body: JSON.stringify({ is_visible: newVisibility }),
      });
      if (reviewsFilter !== 'all') {
        await loadAdminReviews(adminReviewsPage);
      } else {
        adminReviews = adminReviews.map(r => r.id === review.id ? updated : r);
      }
      toast.success(`Ulasan berhasil ${newVisibility ? 'ditampilkan' : 'disembunyikan'}!`);
    } catch (err: any) {
      toast.error(err.message || 'Gagal mengubah status visibilitas ulasan');
    }
  }

  async function handleCreateProduct(e: Event) {
    e.preventDefault();
    isUploading = true;

    try {
      let imageUrl: string | null = null;

      // 1. Upload cover image jika ada
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

      // 2. Buat produk di katalog
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
        // Fallback endpoint
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
      await loadData();
    } catch (err: any) {
      toast.error(err.message || 'Gagal membuat produk.');
    } finally {
      isUploading = false;
    }
  }

  async function updateOrderStatus(orderId: string, nextStatus: string) {
    let trackingNumber: string | null = null;
    if (nextStatus === 'SHIPPED') {
      trackingNumber = prompt('Masukkan Nomor Resi / Tracking Pengiriman:');
      if (!trackingNumber) return;
    }

    try {
      await fetchAdmin(`/orders/${orderId}/status`, {
        method: 'PUT',
        body: JSON.stringify({
          status: nextStatus,
          tracking_number: trackingNumber,
        }),
      });
      toast.success(`Status pesanan #${orderId.slice(0, 8)} diubah ke ${nextStatus}`);
      await loadData();
    } catch (err: any) {
      toast.error(err.message || 'Gagal mengubah status pesanan');
    }
  }

  async function editTrackingNumber(orderId: string, currentResi: string | null | undefined) {
    const newResi = prompt('Masukkan Nomor Resi Pengiriman Baru:', currentResi || '');
    if (!newResi || newResi.trim() === '') return;
    try {
      await fetchAdmin(`/orders/${orderId}/tracking`, {
        method: 'PATCH',
        body: JSON.stringify({ tracking_number: newResi.trim() }),
      });
      toast.success('Nomor resi berhasil diperbarui');
      await loadData();
    } catch (err: any) {
      toast.error(err.message || 'Gagal memperbarui nomor resi');
    }
  }

  onMount(() => {
    if (adminAuth.token) {
      loadData();
    }
  });

  $effect(() => {
    if (adminAuth.token && subTab) {
      loadData();
    }
  });
</script>

<div class="admin-wrapper">
  {#if !adminAuth.token}
    <!-- Admin Login Screen -->
    <div class="admin-login-card">
      <div class="login-header">
        <span class="lock-icon">🔒</span>
        <h2>Program1 Admin Hub</h2>
        <p>Masuk untuk mengelola produk, inventori, dan pemrosesan pesanan.</p>
      </div>

      <form onsubmit={(e) => { e.preventDefault(); adminAuth.login(loginUser, loginPass); }} class="login-form">
        <label>
          Username Admin:
          <input type="text" bind:value={loginUser} required />
        </label>
        <label>
          Password:
          <input type="password" bind:value={loginPass} required />
        </label>

        <button type="submit" class="btn-submit" disabled={adminAuth.isLoading}>
          {adminAuth.isLoading ? 'Memverifikasi...' : 'Masuk ke Dashboard'}
        </button>
      </form>
    </div>
  {:else}
    <!-- Admin Dashboard -->
    <div class="admin-container">
      <aside class="sidebar">
        <div class="side-header">
          <div class="side-brand-row">
            <h3>⚡ {i18n.t('admin.hub_title', 'Admin Hub')}</h3>
            <div class="header-actions-right">
              <NotificationBell role="admin" onNotificationClick={handleAdminNotificationClick} />
              <div class="admin-lang-pills" role="group" aria-label={i18n.t('common.language', 'Pilih Bahasa')}>
                <button
                  class="btn-admin-lang"
                  class:active={i18n.current === 'id'}
                  onclick={() => i18n.setLanguage('id')}
                  type="button"
                  title="Bahasa Indonesia"
                >
                  ID
                </button>
                <button
                  class="btn-admin-lang"
                  class:active={i18n.current === 'en'}
                  onclick={() => i18n.setLanguage('en')}
                  type="button"
                  title="English"
                >
                  EN
                </button>
              </div>
            </div>
          </div>
          <span class="admin-name">User: <strong>{adminAuth.username}</strong></span>
        </div>

        <nav class="side-nav">
          <button class:active={subTab === 'kpi'} onclick={() => subTab = 'kpi'}>📊 {i18n.t('admin.kpi_summary', 'Ringkasan KPI')}</button>
          <button class:active={subTab === 'reports'} onclick={() => { subTab = 'reports'; if (!salesReportData) loadSalesReport(); }}>📑 {i18n.t('admin.sales_reports', 'Laporan Penjualan')}</button>
          <button class:active={subTab === 'catalog'} onclick={() => subTab = 'catalog'}>📦 {i18n.t('admin.catalog_mgmt', 'Katalog Produk')} ({products.length})</button>
          <button class:active={subTab === 'categories'} onclick={() => { subTab = 'categories'; fetchCategories(); }}>🏷️ {i18n.t('admin.category_mgmt', 'Kategori Produk')} ({categories.length})</button>
          <button class:active={subTab === 'inventory'} onclick={() => subTab = 'inventory'}>🏭 {i18n.t('admin.inventory_mgmt', 'Stok & Inventori')}</button>
          <button class:active={subTab === 'orders'} onclick={() => subTab = 'orders'}>🛒 {i18n.t('admin.orders_mgmt', 'Pesanan Masuk')} ({orders.length})</button>
          <button class:active={subTab === 'coupons'} onclick={() => subTab = 'coupons'}>🏷️ {i18n.t('admin.coupons_mgmt', 'Kupon Diskon')} ({coupons.length})</button>
          <button class:active={subTab === 'reviews'} onclick={() => { subTab = 'reviews'; loadAdminReviews(1); }}>⭐ {i18n.t('admin.reviews_mgmt', 'Moderasi Ulasan')} ({adminReviewsTotal})</button>
          <button class:active={subTab === 'returns'} onclick={() => { subTab = 'returns'; loadAdminReturns(); }}>🔄 {i18n.t('admin.returns_mgmt', 'Retur & Refund')} ({adminReturns.length})</button>
          <button class:active={subTab === 'backup'} onclick={() => { subTab = 'backup'; loadBackupsAndHealth(); }}>💾 {i18n.t('admin.database_backup', 'Database & Backup')}</button>
          <button class:active={subTab === 'audit'} onclick={() => subTab = 'audit'}>📜 {i18n.t('admin.audit_logs', 'Audit Logs')}</button>
        </nav>

        <button class="btn-logout" onclick={() => adminAuth.logout()}>🚪 {i18n.t('nav.logout', 'Keluar')}</button>
      </aside>

      <section class="admin-main">
        {#if subTab === 'kpi'}
          <div class="kpi-panel">
            <h2>📊 Ringkasan Kinerja Toko</h2>
            <div class="kpi-grid">
              <div class="kpi-card">
                <span class="kpi-label">Total Produk Aktif</span>
                <strong class="kpi-val">{products.length}</strong>
                <small class="kpi-hint">Tersedia di etalase pembeli</small>
              </div>
              <div class="kpi-card">
                <span class="kpi-label">Total Pesanan Masuk</span>
                <strong class="kpi-val">{orders.length}</strong>
                <small class="kpi-hint">Seluruh transaksi pembeli</small>
              </div>
              <div class="kpi-card">
                <span class="kpi-label">Total Omset Penjualan</span>
                <strong class="kpi-val">
                  {formatRupiah(orders.reduce((sum, o) => sum + o.total_amount_cents, 0))}
                </strong>
                <small class="kpi-hint">Akumulasi nilai pesanan</small>
              </div>
              <div class="kpi-card">
                <span class="kpi-label">Kupon Promo Aktif</span>
                <strong class="kpi-val">{coupons.filter(c => c.is_active).length}</strong>
                <small class="kpi-hint">Dari total {coupons.length} voucher</small>
              </div>
              <div class="kpi-card">
                <span class="kpi-label">Ulasan Pelanggan</span>
                <strong class="kpi-val">{adminReviews.length}</strong>
                <small class="kpi-hint">{adminReviews.filter(r => r.is_visible).length} tampil publik</small>
              </div>
            </div>

            <div style="margin-top: 1.5rem; text-align: right;">
              <button class="btn-action-report btn-export" onclick={() => { subTab = 'reports'; loadSalesReport(); }}>
                📑 Buka Laporan Penjualan & Ekspor Transaksi →
              </button>
            </div>
          </div>

        {:else if subTab === 'reports'}
          <div class="section-panel report-panel">
            <div class="panel-top">
              <div>
                <h2>📑 Laporan Penjualan Transaksi</h2>
                <span class="panel-subtitle">Analisis detail pesanan, filter rentang tanggal, ekspor CSV, dan cetak PDF</span>
              </div>
              <div class="report-actions no-print">
                <button class="btn-action-report btn-export" onclick={exportCSV} disabled={reportLoading || !salesReportData?.rows.length}>
                  📥 Export CSV
                </button>
                <button class="btn-action-report btn-print" onclick={() => window.print()} disabled={reportLoading || !salesReportData?.rows.length}>
                  🖨️ Cetak / Print PDF
                </button>
              </div>
            </div>

            <!-- Filter Controls -->
            <div class="report-filter-bar no-print">
              <div class="filter-item">
                <label for="reportDateFrom">Dari Tanggal:</label>
                <input type="date" id="reportDateFrom" bind:value={reportDateFrom} class="input-date" />
              </div>
              <div class="filter-item">
                <label for="reportDateTo">Sampai Tanggal:</label>
                <input type="date" id="reportDateTo" bind:value={reportDateTo} class="input-date" />
              </div>
              <div class="filter-item">
                <label for="reportStatus">Status Order:</label>
                <select id="reportStatus" bind:value={reportStatusFilter} class="select-status">
                  <option value="all">Semua Status</option>
                  <option value="delivered">Delivered</option>
                  <option value="shipped">Shipped</option>
                  <option value="processing">Processing</option>
                  <option value="paid">Paid</option>
                  <option value="pending">Pending</option>
                  <option value="cancelled">Cancelled</option>
                </select>
              </div>
              <button class="btn-apply-filter" onclick={loadSalesReport} disabled={reportLoading}>
                {reportLoading ? 'Memuat...' : '🔍 Tampilkan'}
              </button>
            </div>

            <!-- Summary KPI Cards -->
            {#if salesReportData}
              <div class="report-kpi-grid">
                <div class="kpi-card">
                  <span class="kpi-label">Total Pendapatan</span>
                  <strong class="kpi-val text-emerald">{formatRupiah(salesReportData.summary.total_revenue_cents / 100)}</strong>
                  <small class="kpi-hint">Akumulasi nilai transaksi</small>
                </div>
                <div class="kpi-card">
                  <span class="kpi-label">Jumlah Pesanan</span>
                  <strong class="kpi-val">{salesReportData.summary.total_orders}</strong>
                  <small class="kpi-hint">Total transaksi periode ini</small>
                </div>
                <div class="kpi-card">
                  <span class="kpi-label">Rata-Rata Order (AOV)</span>
                  <strong class="kpi-val text-cyan">{formatRupiah(salesReportData.summary.average_order_value_cents / 100)}</strong>
                  <small class="kpi-hint">Rata-rata per transaksi</small>
                </div>
                <div class="kpi-card">
                  <span class="kpi-label">Total Item Terjual</span>
                  <strong class="kpi-val text-amber">{salesReportData.summary.total_items_sold} pcs</strong>
                  <small class="kpi-hint">Kuantitas produk keluar</small>
                </div>
              </div>

              <!-- Transaction Detail Table -->
              <div class="table-container report-table-wrap">
                <table class="admin-table report-table">
                  <thead>
                    <tr>
                      <th>Tanggal</th>
                      <th>Order ID</th>
                      <th>Pembeli</th>
                      <th>Detail Produk</th>
                      <th class="text-right">Qty</th>
                      <th class="text-right">Subtotal</th>
                      <th class="text-right">Ongkir</th>
                      <th class="text-right">Diskon</th>
                      <th class="text-right">Total Transaksi</th>
                      <th>Status</th>
                      <th>Pembayaran</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#if salesReportData.rows.length === 0}
                      <tr>
                        <td colspan="11" class="empty-state">Tidak ada transaksi yang cocok dengan kriteria filter.</td>
                      </tr>
                    {:else}
                      {#each salesReportData.rows as row}
                        <tr>
                          <td class="text-nowrap">{new Date(row.order_date).toLocaleDateString('id-ID', { year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })}</td>
                          <td><span class="order-id-badge">#{row.order_id.slice(0, 8)}</span></td>
                          <td>
                            <strong>{row.buyer_name}</strong>
                            {#if row.buyer_email}
                              <div class="text-muted">{row.buyer_email}</div>
                            {/if}
                          </td>
                          <td>
                            <ul class="report-item-list">
                              {#each row.items as it}
                                <li>{it.product_name} <span class="text-muted">x{it.quantity}</span></li>
                              {/each}
                            </ul>
                          </td>
                          <td class="text-right font-mono">{row.items.reduce((s, i) => s + i.quantity, 0)}</td>
                          <td class="text-right font-mono">{formatRupiah(row.subtotal_cents / 100)}</td>
                          <td class="text-right font-mono">{formatRupiah(row.shipping_cents / 100)}</td>
                          <td class="text-right font-mono text-rose">-{formatRupiah(row.discount_cents / 100)}</td>
                          <td class="text-right font-mono font-bold text-emerald">{formatRupiah(row.total_cents / 100)}</td>
                          <td>
                            <span class="status-pill {row.status.toLowerCase()}">{row.status}</span>
                          </td>
                          <td>
                            <span class="status-pill payment-{row.payment_status.toLowerCase()}">{row.payment_status}</span>
                          </td>
                        </tr>
                      {/each}
                    {/if}
                  </tbody>
                </table>
              </div>
            {:else if reportLoading}
              <div class="empty-state">Memuat data laporan penjualan...</div>
            {/if}
          </div>

        {:else if subTab === 'catalog'}
          <div class="section-panel">
            <div class="panel-top">
              <h2>📦 {i18n.t('admin.catalog_mgmt', 'Manajemen Katalog Produk')}</h2>
              <button class="btn-add-prod" onclick={() => isAddProductOpen = true}>+ {i18n.t('admin.add_product', 'Tambah Produk Baru')}</button>
            </div>

            {#if loading}
              <p>Memuat katalog...</p>
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

        {:else if subTab === 'categories'}
          <div class="section-panel">
            <div class="panel-top">
              <div>
                <h2>🏷️ {i18n.t('admin.category_mgmt', 'Manajemen Kategori Produk')}</h2>
                <span class="panel-subtitle">Kelola taksonomi, ikon emoji, urutan, dan pengelompokan produk toko</span>
              </div>
              <button class="btn-add-prod" onclick={() => isAddCategoryOpen = true}>+ {i18n.t('admin.add_category', 'Tambah Kategori Baru')}</button>
            </div>

            {#if categoriesLoading || loading}
              <p>Memuat kategori...</p>
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

        {:else if subTab === 'orders'}
          <div class="section-panel">
            <h2>🛒 Pemrosesan Pesanan Pembeli</h2>
            {#if loading}
              <p>Memuat daftar pesanan...</p>
            {:else if orders.length === 0}
              <p class="empty-text">Belum ada pesanan dari pembeli.</p>
            {:else}
              <table class="table-custom">
                <thead>
                  <tr>
                    <th>ID Pesanan</th>
                    <th>Waktu</th>
                    <th>Total</th>
                    <th>Resi</th>
                    <th>Status</th>
                    <th>Aksi Fulfillment</th>
                  </tr>
                </thead>
                <tbody>
                  {#each orders as o (o.id)}
                    <tr>
                      <td><code>#{o.id.slice(0, 8)}</code></td>
                      <td>{new Date(o.created_at).toLocaleDateString('id-ID')}</td>
                      <td><strong>{formatRupiah(o.total_amount_cents)}</strong></td>
                      <td><small>{o.tracking_number || '-'}</small></td>
                      <td><span class="status-pill {o.status.toLowerCase()}">{o.status}</span></td>
                      <td>
                        {#if o.status === 'PAID'}
                          <button class="btn-action" onclick={() => updateOrderStatus(o.id, 'PROCESSING')}>Proses</button>
                        {:else if o.status === 'PROCESSING'}
                          <button class="btn-action primary" onclick={() => updateOrderStatus(o.id, 'SHIPPED')}>Kirim & Input Resi</button>
                        {:else if o.status === 'SHIPPED'}
                          <button class="btn-action" onclick={() => editTrackingNumber(o.id, o.tracking_number)}>Edit Resi</button>
                        {:else}
                          <span class="done-mark">✓</span>
                        {/if}
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          </div>

        {:else if subTab === 'inventory'}
          <div class="section-panel">
            <h2>🏭 Monitoring Stok & Inventori</h2>
            <table class="table-custom">
              <thead>
                <tr>
                  <th>Nama Barang</th>
                  <th>Stok Tersedia</th>
                  <th>Kondisi</th>
                </tr>
              </thead>
              <tbody>
                {#each products as p (p.id)}
                  <tr>
                    <td><strong>{p.name}</strong></td>
                    <td>{p.stock} unit</td>
                    <td>
                      {#if p.stock <= 0}
                        <span class="badge-crit">Habis</span>
                      {:else if p.stock <= 5}
                        <span class="badge-warn">Kritis (&le;5)</span>
                      {:else}
                        <span class="badge-safe">Aman</span>
                      {/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>

        {:else if subTab === 'audit'}
          <div class="section-panel">
            <h2>📜 Riwayat Audit Trail Sistem</h2>
            {#if auditLogs.length === 0}
              <p class="empty-text">Belum ada log audit tercatat.</p>
            {:else}
              <table class="table-custom">
                <thead>
                  <tr>
                    <th>Waktu</th>
                    <th>Aksi</th>
                    <th>Tipe Resource</th>
                  </tr>
                </thead>
                <tbody>
                  {#each auditLogs as a (a.id)}
                    <tr>
                      <td><small>{new Date(a.created_at).toLocaleString('id-ID')}</small></td>
                      <td><code>{a.action}</code></td>
                      <td>{a.resource_type}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          </div>

        {:else if subTab === 'coupons'}
          <div class="section-panel">
            <div class="panel-top">
              <h2>🏷️ {i18n.t('admin.coupons_mgmt', 'Manajemen Kupon & Kode Promo')}</h2>
              <button class="btn-add-prod" onclick={() => isAddCouponOpen = true}>+ {i18n.t('admin.add_coupon', 'Buat Kupon Baru')}</button>
            </div>

            {#if loading}
              <p>Memuat daftar kupon...</p>
            {:else if coupons.length === 0}
              <p class="empty-text">Belum ada kupon diskon. Klik tombol di atas untuk membuat kupon promo pertama!</p>
            {:else}
              <table class="table-custom">
                <thead>
                  <tr>
                    <th>Kode</th>
                    <th>Tipe & Diskon</th>
                    <th>Min. Belanja</th>
                    <th>Maks. Potongan</th>
                    <th>Pemakaian</th>
                    <th>Periode Berlaku</th>
                    <th>Status</th>
                    <th>Aksi</th>
                  </tr>
                </thead>
                <tbody>
                  {#each coupons as c (c.id)}
                    <tr>
                      <td><strong class="coupon-code-badge">{c.code}</strong></td>
                      <td>
                        {#if c.discount_type === 'PERCENTAGE'}
                          <span class="badge-tag discount-pct">{c.discount_value}% OFF</span>
                        {:else}
                          <span class="badge-tag discount-fix">{formatRupiah(c.discount_value)} OFF</span>
                        {/if}
                      </td>
                      <td>{c.min_order_amount > 0 ? formatRupiah(c.min_order_amount) : 'Tanpa Min.'}</td>
                      <td>{c.max_discount_amount ? formatRupiah(c.max_discount_amount) : '-'}</td>
                      <td>
                        <span class="usage-count">{c.usage_count}</span>
                        <span class="usage-limit">/ {c.usage_limit != null ? c.usage_limit : '∞'}</span>
                      </td>
                      <td>
                        <small>{new Date(c.start_date).toLocaleDateString('id-ID')} - {new Date(c.end_date).toLocaleDateString('id-ID')}</small>
                      </td>
                      <td>
                        <button
                          class="btn-toggle"
                          class:active={c.is_active}
                          onclick={() => handleToggleCoupon(c.id)}
                        >
                          {c.is_active ? '✓ Aktif' : '✕ Nonaktif'}
                        </button>
                      </td>
                      <td>
                        <button class="btn-var-del" onclick={() => handleDeleteCoupon(c.id, c.code)}>Hapus</button>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          </div>

        {:else if subTab === 'reviews'}
          <div class="section-panel">
            <div class="panel-top">
              <div>
                <h2>⭐ Moderasi Ulasan & Rating Produk</h2>
                <span class="panel-subtitle">Total: <strong>{adminReviewsTotal}</strong> ulasan &bull; Halaman {adminReviewsPage} dari {adminReviewsTotalPages}</span>
              </div>
              <div class="filter-group">
                <button
                  class="btn-filter"
                  class:active={reviewsFilter === 'all'}
                  onclick={() => setReviewsFilter('all')}
                >
                  Semua
                </button>
                <button
                  class="btn-filter"
                  class:active={reviewsFilter === 'visible'}
                  onclick={() => setReviewsFilter('visible')}
                >
                  Tampil
                </button>
                <button
                  class="btn-filter"
                  class:active={reviewsFilter === 'hidden'}
                  onclick={() => setReviewsFilter('hidden')}
                >
                  Disembunyikan
                </button>
              </div>
            </div>

            {#if reviewsLoading || loading}
              <p>Memuat daftar ulasan...</p>
            {:else if adminReviews.length === 0}
              <p class="empty-text">Tidak ada ulasan ditemukan.</p>
            {:else}
              <table class="table-custom">
                <thead>
                  <tr>
                    <th>Produk</th>
                    <th>Pembeli</th>
                    <th>Rating</th>
                    <th>Isi Ulasan</th>
                    <th>Tanggal</th>
                    <th>Status</th>
                    <th>Aksi Moderasi</th>
                  </tr>
                </thead>
                <tbody>
                  {#each adminReviews as rev (rev.id)}
                    <tr>
                      <td>
                        <strong>{products.find(p => p.id === rev.product_id)?.name || 'Produk'}</strong>
                        <div><small><code>#{rev.product_id.slice(0, 8)}</code></small></div>
                      </td>
                      <td>
                        <strong>{rev.buyer_name}</strong>
                      </td>
                      <td>
                        <span class="stars-badge">
                          {"★".repeat(rev.rating)}{"☆".repeat(5 - rev.rating)} ({rev.rating}/5)
                        </span>
                      </td>
                      <td class="review-text-cell">
                        {#if rev.review_text}
                          <span>{rev.review_text}</span>
                        {:else}
                          <em class="text-muted">(Tanpa teks ulasan)</em>
                        {/if}
                      </td>
                      <td>
                        <small>{new Date(rev.created_at).toLocaleDateString('id-ID')}</small>
                      </td>
                      <td>
                        <span class="status-pill {rev.is_visible ? 'delivered' : 'cancelled'}">
                          {rev.is_visible ? '✓ Tampil' : '✕ Disembunyikan'}
                        </span>
                      </td>
                      <td>
                        <button
                          class="btn-action"
                          class:primary={!rev.is_visible}
                          onclick={() => handleToggleReviewVisibility(rev)}
                        >
                          {rev.is_visible ? 'Sembunyikan' : 'Tampilkan'}
                        </button>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>

              {#if adminReviewsTotalPages > 1}
                <div class="pagination-bar">
                  <button
                    class="btn-page"
                    disabled={adminReviewsPage <= 1 || reviewsLoading}
                    onclick={() => changeAdminReviewsPage(adminReviewsPage - 1)}
                  >
                    &larr; Sebelumnya
                  </button>
                  <span class="page-info">
                    Halaman {adminReviewsPage} dari {adminReviewsTotalPages} ({adminReviewsTotal} ulasan)
                  </span>
                  <button
                    class="btn-page"
                    disabled={adminReviewsPage >= adminReviewsTotalPages || reviewsLoading}
                    onclick={() => changeAdminReviewsPage(adminReviewsPage + 1)}
                  >
                    Selanjutnya &rarr;
                  </button>
                </div>
              {/if}
            {/if}
          </div>
        {:else if subTab === 'returns'}
          <div class="card-panel">
            <div class="panel-header">
              <h2>🔄 Manajemen Retur & Pengembalian Dana (Refund)</h2>
              <div class="header-actions">
                <button class="btn-refresh" onclick={loadAdminReturns} disabled={adminReturnsLoading}>
                  {adminReturnsLoading ? 'Memuat...' : '🔄 Refresh'}
                </button>
              </div>
            </div>

            <!-- Status Filter Pills -->
            <div class="filter-pills-row" style="margin-bottom: 1rem; display: flex; gap: 0.5rem; flex-wrap: wrap;">
              <button class="pill-btn" class:active={returnStatusFilter === ''} onclick={() => { returnStatusFilter = ''; loadAdminReturns(); }}>
                Semua ({adminReturns.length})
              </button>
              <button class="pill-btn" class:active={returnStatusFilter === 'pending'} onclick={() => { returnStatusFilter = 'pending'; loadAdminReturns(); }}>
                Menunggu Review
              </button>
              <button class="pill-btn" class:active={returnStatusFilter === 'approved'} onclick={() => { returnStatusFilter = 'approved'; loadAdminReturns(); }}>
                Disetujui
              </button>
              <button class="pill-btn" class:active={returnStatusFilter === 'return_shipped'} onclick={() => { returnStatusFilter = 'return_shipped'; loadAdminReturns(); }}>
                Dalam Pengiriman
              </button>
              <button class="pill-btn" class:active={returnStatusFilter === 'received'} onclick={() => { returnStatusFilter = 'received'; loadAdminReturns(); }}>
                Diterima & Restock
              </button>
              <button class="pill-btn" class:active={returnStatusFilter === 'refunded'} onclick={() => { returnStatusFilter = 'refunded'; loadAdminReturns(); }}>
                Selesai (Refunded)
              </button>
              <button class="pill-btn" class:active={returnStatusFilter === 'rejected'} onclick={() => { returnStatusFilter = 'rejected'; loadAdminReturns(); }}>
                Ditolak
              </button>
            </div>

            {#if adminReturnsLoading}
              <p>Memuat daftar permintaan retur...</p>
            {:else if adminReturns.length === 0}
              <p class="empty-text">Belum ada pengajuan retur untuk status yang dipilih.</p>
            {:else}
              <table class="table-custom">
                <thead>
                  <tr>
                    <th>ID / Tanggal</th>
                    <th>ID Order</th>
                    <th>ID Pembeli</th>
                    <th>Alasan Retur</th>
                    <th>Estimasi Refund</th>
                    <th>Status</th>
                    <th>Bukti</th>
                    <th>Aksi Admin</th>
                  </tr>
                </thead>
                <tbody>
                  {#each adminReturns as r (r.id)}
                    <tr>
                      <td>
                        <strong style="color: #38bdf8;">#{r.id.slice(0, 8)}</strong>
                        <br />
                        <small>{new Date(r.created_at).toLocaleDateString('id-ID')}</small>
                      </td>
                      <td>
                        <code>#{r.order_id.slice(0, 8)}</code>
                      </td>
                      <td>
                        <small>{r.buyer_id.slice(0, 12)}</small>
                      </td>
                      <td>
                        <strong>{r.reason}</strong>
                        {#if r.description}
                          <p style="font-size: 0.8rem; color: #94a3b8; margin: 0.2rem 0 0 0;">{r.description}</p>
                        {/if}
                      </td>
                      <td>
                        <strong style="color: #34d399;">{formatRupiah(r.refund_amount_cents)}</strong>
                      </td>
                      <td>
                        <span class="status-pill {r.status.toLowerCase()}">
                          {r.status.toUpperCase()}
                        </span>
                      </td>
                      <td>
                        {#if r.evidence_urls && r.evidence_urls.length > 0}
                          <a href={r.evidence_urls[0]} target="_blank" rel="noopener noreferrer" style="color: #38bdf8; font-size: 0.82rem; text-decoration: none;">
                            🖼️ Lihat Foto
                          </a>
                        {:else}
                          <span style="color: #64748b;">-</span>
                        {/if}
                      </td>
                      <td>
                        <div style="display: flex; gap: 0.35rem; flex-wrap: wrap;">
                          {#if r.status === 'pending'}
                            <button
                              class="btn-action primary"
                              onclick={() => processReturn(r.id, 'approve')}
                              disabled={isProcessingReturn}
                            >
                              ✓ Setujui
                            </button>
                            <button
                              class="btn-action danger"
                              onclick={() => openRejectModal(r.id)}
                              disabled={isProcessingReturn}
                            >
                              ✕ Tolak
                            </button>
                          {:else if r.status === 'approved'}
                            <button
                              class="btn-action primary"
                              onclick={() => updateReturnStatus(r.id, 'return_shipped')}
                            >
                              🚚 Pengiriman
                            </button>
                          {:else if r.status === 'return_shipped'}
                            <button
                              class="btn-action primary"
                              onclick={() => updateReturnStatus(r.id, 'received')}
                            >
                              📦 Terima & Restock
                            </button>
                          {:else if r.status === 'received'}
                            <button
                              class="btn-action success"
                              onclick={() => updateReturnStatus(r.id, 'refunded')}
                            >
                              💸 Refund Selesai
                            </button>
                          {:else}
                            <small style="color: #64748b;">Proses Selesai</small>
                          {/if}
                        </div>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          </div>
        {:else if subTab === 'backup'}
          <div class="card-panel">
            <div class="panel-header">
              <h2>💾 Database & Backup Manager</h2>
              <div class="header-actions">
                <button
                  class="btn-refresh"
                  onclick={loadBackupsAndHealth}
                  disabled={backupLoading}
                >
                  {backupLoading ? 'Memeriksa...' : '🔄 Refresh Status'}
                </button>
                <button
                  class="btn-add-prod"
                  onclick={handleCreateBackup}
                  disabled={creatingBackup}
                >
                  {creatingBackup ? 'Membuat Backup...' : '⚡ Buat Backup Sekarang'}
                </button>
              </div>
            </div>

            <!-- Health Status Banner -->
            {#if dbHealth}
              <div class="db-health-banner" style="display: flex; gap: 1.5rem; align-items: center; background: #0f172a; border: 1px solid #1e293b; border-radius: 8px; padding: 1rem 1.25rem; margin-bottom: 1.5rem;">
                <div style="font-size: 2rem;">
                  {dbHealth.status === 'healthy' ? '🟢' : '🟡'}
                </div>
                <div style="flex: 1;">
                  <div style="font-weight: 700; font-size: 1.05rem; color: #f8fafc; margin-bottom: 0.25rem;">
                    Status Database: <span style="color: {dbHealth.status === 'healthy' ? '#4ade80' : '#facc15'}; text-transform: uppercase;">{dbHealth.status}</span>
                  </div>
                  <div style="font-size: 0.85rem; color: #94a3b8; display: flex; gap: 1.5rem;">
                    <span>Engine: <strong style="color: #cbd5e1;">{dbHealth.engine}</strong></span>
                    <span>Integritas: <strong style="color: #cbd5e1;">PRAGMA integrity_check = {dbHealth.integrity}</strong></span>
                  </div>
                </div>
              </div>
            {/if}

            <h3 style="font-size: 1rem; color: #94a3b8; margin-bottom: 0.75rem;">📁 Riwayat Berkas Backup Snapshot ({backups.length})</h3>

            {#if backupLoading}
              <p>Memuat daftar backup...</p>
            {:else if backups.length === 0}
              <div class="empty-state" style="text-align: center; padding: 3rem 1rem; color: #64748b;">
                <div style="font-size: 3rem; margin-bottom: 0.5rem;">📭</div>
                <p>Belum ada berkas backup yang tersimpan di server.</p>
                <p style="font-size: 0.85rem;">Klik tombol "⚡ Buat Backup Sekarang" di kanan atas untuk membuat snapshot SQLite instan.</p>
              </div>
            {:else}
              <table class="table-custom">
                <thead>
                  <tr>
                    <th>Nama Berkas</th>
                    <th>Ukuran</th>
                    <th>Waktu Dibuat</th>
                    <th style="text-align: right;">Aksi</th>
                  </tr>
                </thead>
                <tbody>
                  {#each backups as b (b.filename)}
                    <tr>
                      <td>
                        <strong style="color: #38bdf8; font-family: monospace;">{b.filename}</strong>
                      </td>
                      <td>{(b.size_bytes / 1024).toFixed(1)} KB</td>
                      <td>
                        <small>{new Date(b.created_at).toLocaleString('id-ID')}</small>
                      </td>
                      <td style="text-align: right;">
                        <button
                          class="btn-action primary"
                          onclick={() => downloadBackup(b.filename)}
                        >
                          ⬇️ Unduh Berkas
                        </button>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          </div>
        {/if}
      </section>
    </div>
  {/if}

  <!-- Reject Return Modal -->
  {#if rejectModalOpen}
    <div class="modal-overlay" onclick={() => rejectModalOpen = false} role="button" tabindex="0" onkeydown={(e) => e.key === 'Escape' && (rejectModalOpen = false)}>
      <div class="modal-card" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" style="max-width: 440px;">
        <div class="modal-header">
          <h3>✕ Alasan Penolakan Retur</h3>
          <button class="close-btn" onclick={() => rejectModalOpen = false}>&times;</button>
        </div>
        <div style="display: flex; flex-direction: column; gap: 0.85rem; margin-top: 1rem;">
          <label style="font-size: 0.85rem; color: #cbd5e1; font-weight: 600;">
            Berikan catatan/alasan ke pembeli (wajib):
            <textarea
              bind:value={rejectNotes}
              rows="3"
              placeholder="Contoh: Garansi resmi telah lewat / Kerusakan akibat kesalahan pengguna."
              style="width: 100%; margin-top: 0.35rem; background: #1e293b; border: 1px solid #334155; border-radius: 6px; padding: 0.6rem; color: #fff;"
            ></textarea>
          </label>
          <div style="display: flex; justify-content: flex-end; gap: 0.5rem; margin-top: 0.5rem;">
            <button type="button" class="btn-action" onclick={() => rejectModalOpen = false}>Batal</button>
            <button
              type="button"
              class="btn-action danger"
              onclick={() => processReturn(rejectTargetId, 'reject', rejectNotes)}
              disabled={isProcessingReturn || !rejectNotes.trim()}
            >
              {isProcessingReturn ? 'Memproses...' : 'Tolak Retur'}
            </button>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Add Product Modal -->
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

  <!-- Add Category Modal -->
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

  <!-- Edit Category Modal -->
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

        <!-- Daftar Varian Saat Ini -->
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

        <!-- Form Tambah Varian Baru -->
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

  <!-- Modal Buat Kupon Diskon -->
  {#if isAddCouponOpen}
    <div class="modal-overlay" onclick={() => isAddCouponOpen = false} role="button" tabindex="0" onkeydown={(e) => e.key === 'Escape' && (isAddCouponOpen = false)}>
      <div class="modal-card" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
        <div class="modal-header">
          <h3>🏷️ Buat Kupon Diskon Baru</h3>
          <button class="close-btn" onclick={() => isAddCouponOpen = false}>&times;</button>
        </div>

        <form onsubmit={handleCreateCoupon} class="prod-form">
          <div class="row-fields">
            <label>
              Kode Kupon (Otomatis Kapital):
              <input type="text" bind:value={newCouponCode} required placeholder="Contoh: HEMAT20" style="text-transform: uppercase;" />
            </label>
            <label>
              Tipe Diskon:
              <select bind:value={newCouponDiscountType} class="select-custom">
                <option value="PERCENTAGE">Persentase (%)</option>
                <option value="FIXED_AMOUNT">Nominal Tetap (Rp)</option>
              </select>
            </label>
          </div>

          <div class="row-fields">
            <label>
              Nilai Diskon:
              <input
                type="number"
                bind:value={newCouponDiscountVal}
                min="1"
                max={newCouponDiscountType === 'PERCENTAGE' ? 100 : undefined}
                required
              />
            </label>
            {#if newCouponDiscountType === 'PERCENTAGE'}
              <label>
                Maksimal Potongan (Rp, Opsional):
                <input type="number" bind:value={newCouponMaxCap} placeholder="Kosongkan jika tanpa batas" min="0" />
              </label>
            {/if}
          </div>

          <div class="row-fields">
            <label>
              Min. Nilai Pesanan (Rp):
              <input type="number" bind:value={newCouponMinOrder} min="0" placeholder="0 = tanpa minimum" />
            </label>
            <label>
              Batas Total Pemakaian (Opsional):
              <input type="number" bind:value={newCouponLimit} min="1" placeholder="Kosongkan = tanpa batas" />
            </label>
          </div>

          <div class="row-fields">
            <label>
              Tanggal Mulai Berlaku:
              <input type="datetime-local" bind:value={newCouponStartDate} required />
            </label>
            <label>
              Tanggal Kedaluwarsa:
              <input type="datetime-local" bind:value={newCouponEndDate} required />
            </label>
          </div>

          <div class="modal-actions">
            <button type="button" class="btn-cancel" onclick={() => isAddCouponOpen = false}>Batal</button>
            <button type="submit" class="btn-save" disabled={isCreatingCoupon}>
              {isCreatingCoupon ? 'Membuat Kupon...' : 'Simpan Kupon 🚀'}
            </button>
          </div>
        </form>
      </div>
    </div>
  {/if}
</div>

<style>
  .admin-wrapper { width: 100%; min-height: 80vh; }
  .admin-login-card {
    max-width: 400px; margin: 3rem auto; background: #0f172a;
    border: 1px solid #334155; border-radius: 12px; padding: 2rem;
    box-shadow: 0 10px 30px rgba(0,0,0,0.5); text-align: center;
  }
  .lock-icon { font-size: 2.5rem; }
  .login-header h2 { margin: 0.5rem 0 0.25rem; color: #38bdf8; }
  .login-header p { font-size: 0.85rem; color: #94a3b8; margin-bottom: 1.5rem; }
  .login-form { display: flex; flex-direction: column; gap: 1rem; text-align: left; }
  .login-form label { font-size: 0.85rem; color: #cbd5e1; display: flex; flex-direction: column; gap: 0.3rem; }
  .login-form input {
    background: #1e293b; border: 1px solid #334155; border-radius: 6px;
    padding: 0.65rem; color: #fff;
  }
  .btn-submit {
    background: #0284c7; color: #fff; border: none; padding: 0.75rem;
    border-radius: 6px; font-weight: bold; cursor: pointer; margin-top: 0.5rem;
  }

  /* Dashboard Layout */
  .admin-container { display: flex; gap: 1.5rem; min-height: 75vh; }
  .sidebar {
    width: 220px; background: #0f172a; border: 1px solid #1e293b;
    border-radius: 12px; padding: 1.25rem; display: flex; flex-direction: column; gap: 1rem;
  }
  .side-header h3 { margin: 0; color: #38bdf8; font-size: 1.15rem; }
  .side-brand-row { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.35rem; }
  .header-actions-right { display: flex; align-items: center; gap: 0.5rem; }
  .admin-lang-pills { display: flex; gap: 2px; background: #1e293b; border-radius: 12px; padding: 2px; border: 1px solid #334155; }
  .btn-admin-lang {
    background: transparent; border: none; color: #94a3b8; font-size: 0.7rem; font-weight: 700;
    padding: 0.15rem 0.45rem; border-radius: 10px; cursor: pointer; transition: all 0.2s;
  }
  .btn-admin-lang:hover { color: #fff; }
  .btn-admin-lang.active { background: #0284c7; color: #fff; }
  .admin-name { font-size: 0.8rem; color: #94a3b8; }
  .side-nav { display: flex; flex-direction: column; gap: 0.4rem; flex: 1; }
  .side-nav button {
    background: transparent; border: none; color: #94a3b8; text-align: left;
    padding: 0.65rem 0.8rem; border-radius: 6px; cursor: pointer; font-size: 0.9rem;
    transition: all 0.2s;
  }
  .side-nav button:hover { background: #1e293b; color: #fff; }
  .side-nav button.active { background: #0284c7; color: #fff; font-weight: bold; }
  .btn-logout {
    background: #7f1d1d; color: #fecaca; border: none; padding: 0.5rem;
    border-radius: 6px; cursor: pointer; font-size: 0.85rem;
  }

  .admin-main { flex: 1; background: #0f172a; border: 1px solid #1e293b; border-radius: 12px; padding: 1.5rem; }
  .kpi-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 1rem; margin-top: 1.25rem; }
  .kpi-card {
    background: #1e293b; border: 1px solid #334155; border-radius: 10px;
    padding: 1.25rem; display: flex; flex-direction: column;
  }
  .kpi-label { font-size: 0.85rem; color: #94a3b8; }
  .kpi-val { font-size: 1.8rem; color: #38bdf8; margin: 0.5rem 0; font-weight: bold; }
  .kpi-hint { font-size: 0.75rem; color: #64748b; }

  .panel-top { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
  .btn-add-prod {
    background: #059669; color: #fff; border: none; padding: 0.55rem 1rem;
    border-radius: 6px; font-weight: bold; cursor: pointer;
  }

  /* Table */
  .table-custom { width: 100%; border-collapse: collapse; margin-top: 1rem; }
  .table-custom th, .table-custom td {
    padding: 0.75rem 1rem; text-align: left; border-bottom: 1px solid #1e293b; font-size: 0.9rem;
  }
  .table-custom th { background: #1e293b; color: #94a3b8; font-size: 0.8rem; }
  .td-img { width: 44px; height: 44px; text-align: center; }
  .td-img img { width: 40px; height: 40px; object-fit: cover; border-radius: 4px; }
  .badge-tag { background: #334155; padding: 0.2rem 0.5rem; border-radius: 4px; font-size: 0.75rem; }
  .badge-stock { color: #10b981; font-weight: bold; }
  .badge-stock.empty { color: #ef4444; }
  .badge-crit { background: #dc2626; color: #fff; padding: 0.2rem 0.4rem; border-radius: 4px; font-size: 0.75rem; }
  .badge-warn { background: #d97706; color: #fff; padding: 0.2rem 0.4rem; border-radius: 4px; font-size: 0.75rem; }
  .badge-safe { background: #059669; color: #fff; padding: 0.2rem 0.4rem; border-radius: 4px; font-size: 0.75rem; }
  .status-pill { padding: 0.2rem 0.5rem; border-radius: 4px; font-size: 0.75rem; font-weight: bold; }
  .status-pill.paid { background: #0284c7; }
  .status-pill.processing { background: #d97706; }
  .status-pill.shipped { background: #7c3aed; }
  .btn-action {
    background: #334155; color: #fff; border: none; padding: 0.35rem 0.7rem;
    border-radius: 4px; cursor: pointer; font-size: 0.8rem;
  }
  .btn-action.primary { background: #0284c7; font-weight: bold; }
  .done-mark { color: #10b981; font-weight: bold; }
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

  /* Variant Management Modal */
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

  /* Coupon Specific Styles */
  .coupon-code-badge {
    background: #1e3a8a; color: #93c5fd; padding: 0.2rem 0.5rem;
    border-radius: 4px; font-family: monospace; font-size: 0.85rem; letter-spacing: 0.5px;
  }
  .discount-pct { background: #065f46; color: #6ee7b7; font-weight: bold; }
  .discount-fix { background: #7c2d12; color: #fdba74; font-weight: bold; }
  .usage-count { color: #38bdf8; font-weight: bold; }
  .usage-limit { color: #64748b; font-size: 0.8rem; }
  .btn-toggle {
    border: none; padding: 0.25rem 0.6rem; border-radius: 4px;
    font-size: 0.75rem; font-weight: bold; cursor: pointer;
    background: #334155; color: #94a3b8; transition: all 0.2s;
  }
  .btn-toggle.active { background: #059669; color: #ecfdf5; }
  .select-custom {
    background: #1e293b; border: 1px solid #334155; border-radius: 6px;
    padding: 0.6rem; color: #fff; font-family: inherit;
  }

  /* Review Moderation Styles */
  .filter-group { display: flex; gap: 0.5rem; }
  .btn-filter {
    background: #1e293b; color: #94a3b8; border: 1px solid #334155;
    padding: 0.35rem 0.75rem; border-radius: 6px; cursor: pointer; font-size: 0.8rem;
    transition: all 0.2s;
  }
  .btn-filter:hover { background: #334155; color: #fff; }
  .btn-filter.active { background: #0284c7; color: #fff; border-color: #0284c7; font-weight: bold; }
  .stars-badge { color: #f59e0b; font-size: 0.85rem; font-weight: 500; }
  .review-text-cell { max-width: 320px; font-size: 0.85rem; line-height: 1.4; color: #cbd5e1; }
  .text-muted { color: #64748b; font-size: 0.8rem; font-style: italic; }
  .status-pill.delivered { background: #065f46; color: #a7f3d0; }
  .status-pill.cancelled { background: #7f1d1d; color: #fecaca; }

  .panel-subtitle { font-size: 0.85rem; color: #94a3b8; margin-top: 0.25rem; display: inline-block; }
  .pagination-bar {
    display: flex; justify-content: space-between; align-items: center;
    padding-top: 1rem; margin-top: 1rem; border-top: 1px solid #334155;
  }
  .btn-page {
    background: #1e293b; color: #cbd5e1; border: 1px solid #334155;
    padding: 0.4rem 0.85rem; border-radius: 6px; font-size: 0.8rem; cursor: pointer;
  }
  .btn-page:hover:not(:disabled) { background: #334155; color: #fff; }
  .btn-page:disabled { opacity: 0.4; cursor: not-allowed; }
  .page-info { font-size: 0.8rem; color: #94a3b8; }

  /* Sales Report Styles */
  .report-filter-bar {
    display: flex; gap: 1rem; align-items: flex-end; flex-wrap: wrap;
    background: #0f172a; padding: 1rem; border-radius: 8px; border: 1px solid #334155; margin-bottom: 1.5rem;
  }
  .filter-item { display: flex; flex-direction: column; gap: 0.35rem; }
  .filter-item label { font-size: 0.8rem; color: #94a3b8; font-weight: 500; }
  .input-date, .select-status {
    background: #1e293b; border: 1px solid #334155; color: #fff;
    padding: 0.45rem 0.75rem; border-radius: 6px; font-size: 0.85rem; font-family: inherit;
  }
  .btn-apply-filter {
    background: #0284c7; color: #fff; border: none; padding: 0.5rem 1.1rem;
    border-radius: 6px; font-weight: bold; cursor: pointer; transition: background 0.2s;
  }
  .btn-apply-filter:hover { background: #0369a1; }
  .btn-apply-filter:disabled { opacity: 0.5; cursor: not-allowed; }
  .report-actions { display: flex; gap: 0.5rem; }
  .btn-action-report {
    border: none; padding: 0.5rem 0.9rem; border-radius: 6px; font-size: 0.85rem;
    font-weight: 600; cursor: pointer; display: flex; align-items: center; gap: 0.4rem; transition: opacity 0.2s;
  }
  .btn-export { background: #059669; color: #fff; }
  .btn-export:hover:not(:disabled) { background: #047857; }
  .btn-print { background: #475569; color: #fff; }
  .btn-print:hover:not(:disabled) { background: #334155; }
  .btn-action-report:disabled { opacity: 0.4; cursor: not-allowed; }
  .report-kpi-grid {
    display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 1rem; margin-bottom: 1.5rem;
  }
  .report-table-wrap { overflow-x: auto; margin-top: 0.5rem; }
  .report-table th, .report-table td { padding: 0.65rem 0.85rem; font-size: 0.85rem; }
  .text-right { text-align: right; }
  .text-nowrap { white-space: nowrap; }
  .text-emerald { color: #34d399; }
  .text-cyan { color: #38bdf8; }
  .text-amber { color: #fbbf24; }
  .text-rose { color: #f87171; }
  .font-mono { font-family: monospace; }
  .report-item-list { list-style: none; margin: 0; padding: 0; font-size: 0.8rem; line-height: 1.35; }
  .status-pill.payment-paid { background: #065f46; color: #a7f3d0; }
  .status-pill.payment-pending { background: #854d0e; color: #fef08a; }
  .status-pill.payment-cancelled { background: #7f1d1d; color: #fecaca; }

  @media print {
    :global(body) { background: #fff !important; color: #000 !important; }
    .no-print, nav, .panel-top button, .report-filter-bar, .report-actions, .kpi-hint { display: none !important; }
    .report-panel { background: #fff !important; color: #000 !important; border: none !important; box-shadow: none !important; }
    .report-table { width: 100%; border-collapse: collapse; color: #000 !important; }
    .report-table th, .report-table td { border: 1px solid #ccc !important; color: #000 !important; padding: 4px 6px !important; font-size: 9pt !important; }
    .kpi-card { border: 1px solid #ccc !important; background: #fff !important; color: #000 !important; padding: 6px !important; }
    .kpi-val, .kpi-label { color: #000 !important; }
  }
</style>
