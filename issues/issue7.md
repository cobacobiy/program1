# Issue #7: Frontend Svelte Komponen Monolitik (App.svelte 1658 baris, AdminHub.svelte 2601 baris)

## Severity: 🟡 LOW (Maintainability / Tech Debt)

## Deskripsi
Dua komponen frontend utama terlalu besar dan sulit di-maintain:

| File | Baris | Size |
|------|-------|------|
| `frontend/src/App.svelte` | 1,658 baris | 62 KB |
| `frontend/src/lib/AdminHub.svelte` | 2,601 baris | 110 KB |

Ini melanggar prinsip **Single Responsibility** dan membuat:
- Debugging jadi sangat sulit (scroll ribuan baris untuk menemukan bug)
- Merge conflict sering terjadi jika 2 developer edit file yang sama
- Re-render performance buruk (perubahan kecil re-render seluruh komponen)
- Code review jadi sangat berat

## File yang Terdampak
- `frontend/src/App.svelte` — 1,658 baris (Storefront SPA)
- `frontend/src/lib/AdminHub.svelte` — 2,601 baris (Admin Dashboard)

## Langkah Perbaikan (Refactor Bertahap)

### Phase 1: Identifikasi Section di App.svelte
Buka `frontend/src/App.svelte` dan identifikasi blok-blok yang bisa diekstrak:

1. **Product Grid / Listing** → `ProductGrid.svelte`
2. **Product Detail / Modal** → `ProductDetail.svelte`
3. **Cart Sidebar** → `CartSidebar.svelte`
4. **Category Filter** → `CategoryFilter.svelte`
5. **Header / Navigation** → `StoreHeader.svelte`
6. **Footer** → `StoreFooter.svelte`

Untuk setiap komponen yang diekstrak:
```svelte
<!-- Contoh: Buat file frontend/src/lib/ProductGrid.svelte -->
<script lang="ts">
  // Pindahkan logic terkait product listing dari App.svelte ke sini
  // Props yang dibutuhkan
  let { products, onProductClick, onAddToCart } = $props();
</script>

<!-- Pindahkan HTML terkait product grid dari App.svelte ke sini -->
```

Kemudian di `App.svelte`, replace blok yang dipindahkan dengan:
```svelte
<ProductGrid
  products={products}
  onProductClick={handleProductClick}
  onAddToCart={handleAddToCart}
/>
```

### Phase 2: Identifikasi Section di AdminHub.svelte
Buka `frontend/src/lib/AdminHub.svelte` dan identifikasi tab/section:

1. **Dashboard Overview** → `AdminDashboard.svelte`
2. **Product/Catalog Management** → `AdminCatalog.svelte`
3. **Order Management** → `AdminOrders.svelte`
4. **Inventory Management** → `AdminInventory.svelte`
5. **User/Staff Management** → `AdminUsers.svelte`
6. **Analytics/Reports** → `AdminAnalytics.svelte`
7. **Settings/Config** → `AdminSettings.svelte`
8. **Chat Management** → `AdminChat.svelte`
9. **Coupon Management** → `AdminCoupons.svelte`
10. **Return Management** → `AdminReturns.svelte`

### Phase 3: Shared State Management
Untuk data yang di-share antar komponen, gunakan Svelte 5 runes (`$state`) dalam file `.svelte.ts`:
- `frontend/src/lib/products.svelte.ts` — product state
- `frontend/src/lib/orders.svelte.ts` — order state
- (sudah ada: `auth.svelte.ts`, `cart.svelte.ts`, `toast.svelte.ts`)

### Aturan Refactor
1. **Jangan ubah fungsionalitas** — hanya pindahkan kode, jangan tambah/hapus fitur
2. **Satu komponen per section** — max 200-300 baris per komponen
3. **Test manual setelah setiap extract** — buka browser, pastikan UI masih berfungsi sama
4. **Commit setelah setiap extract berhasil** — jangan batch terlalu banyak

### Verifikasi
```bash
cd frontend && bun run build
# Pastikan tidak ada error build
```

Lalu test manual di browser untuk setiap section yang di-refactor.

## Estimasi Waktu: 4-8 jam (bisa dikerjakan bertahap, 1-2 komponen per sesi)
## Kompleksitas: Sedang (banyak file tapi straightforward move)
## Prioritas: Bisa dikerjakan belakangan, tidak urgent
