# Issue 23: Dynamic SEO, Open Graph & Sitemap Generator (Phase 4 — #6)

> **Prioritas:** 🟢 MEDIUM  
> **Estimasi:** 2 hari  
> **Kesulitan:** ⭐⭐ Menengah  
> **Prerequisite:** Selesai Issue #38 (Catalog)  

---

## 📋 Deskripsi

Untuk toko e-commerce, kemampuan produk ditemukan di mesin pencari (Google, Bing) dan tampil menawan saat dibagikan (*viral sharing*) di media sosial (WhatsApp, Facebook, Twitter, Telegram) adalah kunci peningkatan trafik organik.

Saat ini antarmuka adalah Svelte SPA dengan satu file `index.html` statis. Ketika link produk dibagikan ke WhatsApp, WhatsApp crawler tidak mengeksekusi JavaScript client-side sehingga preview gambar dan harga tidak muncul.

Issue ini mengimplementasikan:
1. **Open Graph & Twitter Card Meta Tags**: Deteksi crawler sosial media (*social bots*) pada endpoint rute produk `/product/:id` dan menyajikan tag `<meta property="og:title">`, `<meta property="og:image">`, `<meta property="og:description">`, `<meta property="product:price:amount">`.
2. **Dynamic Sitemap (`/sitemap.xml`)**: Endpoint otomatis yang menghasilkan daftar seluruh produk aktif dan kategori dalam format standar XML Sitemap untuk di-crawl oleh Googlebot.
3. **Robots.txt (`/robots.txt`)**: Endpoint konfigurasi perayapan yang mengarahkan crawler publik ke `/sitemap.xml` dan melarang perayapan pada halaman admin (`Disallow: /admin`).

---

## 🎯 Acceptance Criteria

- [ ] Mengakses `/sitemap.xml` menghasilkan XML valid yang memuat URL seluruh produk aktif, kategori, dan beranda dengan tag `<lastmod>` yang sesuai.
- [ ] Mengakses `/robots.txt` menghasilkan instruksi perayap standar dan tautan ke sitemap.
- [ ] Permintaan ke `/product/:id` dari crawler media sosial (terdeteksi via header `User-Agent: WhatsApp/Facebook/Twitterbot/TelegramBot`) menyajikan halaman HTML awal dengan metadata Open Graph lengkap (judul produk, gambar sampul, harga, dan deskripsi) sebelum di-redirect/mount oleh SPA.
- [ ] Tag `<title>` di Svelte SPA otomatis berganti secara dinamis mengikuti nama produk yang sedang dibuka ("Program1 - Sepatu Running Pria").
- [ ] Minimal 3 unit test untuk validasi struktur XML sitemap, robots.txt, dan penyajian Open Graph tags.

---

## 📐 Langkah-Langkah Pengerjaan

### Langkah 1: Handler Sitemap & Robots.txt
**File:** `crates/web/src/handlers/seo.rs`

```rust
use axum::{
    extract::State,
    response::{IntoResponse, Response},
    http::{header, StatusCode},
};
use crate::AppState;

pub async fn get_robots_txt() -> impl IntoResponse {
    let content = "User-agent: *\nAllow: /\nDisallow: /admin\nDisallow: /api/\n\nSitemap: /sitemap.xml\n";
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], content)
}

pub async fn get_sitemap_xml(State(state): State<AppState>) -> Result<Response, (StatusCode, String)> {
    let products = state.catalog.list_products(None, 1, 1000).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    
    // Home
    xml.push_str("  <url><loc>/</loc><priority>1.0</priority></url>\n");

    for p in products.items {
        xml.push_str(&format!(
            "  <url><loc>/#product-{}</loc><lastmod>{}</lastmod><priority>0.8</priority></url>\n",
            p.id, p.updated_at
        ));
    }

    xml.push_str("</urlset>");
    Ok(([(header::CONTENT_TYPE, "application/xml; charset=utf-8")], xml).into_response())
}
```

### Langkah 2: Middleware Bot Detection & OG Tag Injection
**File:** `crates/web/src/middleware/social_crawler.rs`
- Cek jika header `User-Agent` mengandung: `WhatsApp`, `facebookexternalhit`, `Twitterbot`, `TelegramBot`, `Googlebot`.
- Jika URL adalah rute produk, ambil data produk dari database dan kembalikan HTML ringan dengan meta tag OG:
  ```html
  <meta property="og:title" content="{product.name}" />
  <meta property="og:description" content="{product.description}" />
  <meta property="og:image" content="{product.image_url}" />
  <meta property="og:price:amount" content="{product.price_cents / 100}" />
  <meta property="og:price:currency" content="IDR" />
  ```

### Langkah 3: Frontend Dynamic Document Title
**File:** `frontend/src/App.svelte`
- Gunakan `<svelte:head>` untuk mengubah `<title>` dan meta deskripsi secara reaktif ketika modal detail produk dibuka.

### Langkah 4: Unit & Integration Tests
**File:** `crates/web/tests/seo_test.rs`
- Uji output `GET /robots.txt`.
- Uji output `GET /sitemap.xml` memuat produk aktif.
- Uji simulasi User-Agent crawler mendapatkan Open Graph tags.
