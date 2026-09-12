# Issue 20: Full-Text Search Autocomplete & Popular Suggestions (Phase 4 — #3)

> **Prioritas:** 🟡 HIGH  
> **Estimasi:** 2-3 hari  
> **Kesulitan:** ⭐⭐ Menengah  
> **Prerequisite:** Selesai Issue #38 (Catalog Search)  

---

## 📋 Deskripsi

Saat ini pencarian katalog produk menggunakan query SQL sederhana `LIKE '%query%'`.  
Pada toko e-commerce dengan ribuan produk, pengguna mengharapkan:
1. **Pencarian Cepat Cerdas (Full-Text Search)**: Menggunakan SQLite FTS5 / Postgres Full-Text Search untuk pencarian multi-kata berkecepatan tinggi.
2. **Saran Pencarian Instan (Live Autocomplete)**: Saat mengetik di kolom pencarian, muncul dropdown rekomendasi judul produk dan kategori yang cocok secara real-time.
3. **Pencarian Terpopuler (Trending Keywords)**: Menampilkan tag pencarian populer ("Paling sering dicari: Kaos Polos, Sepatu Sneakers, Jaket").
4. **Riwayat Pencarian Lokal**: Menyimpan 5 kata kunci terakhir yang dicari pengguna di browser (`localStorage`).

---

## 🎯 Acceptance Criteria

- [ ] Endpoint `GET /catalog/search/suggest?q=...` mengembalikan maksimal 5 saran judul produk dan 3 nama kategori yang relevan secara instan (< 50ms).
- [ ] Endpoint `GET /catalog/search/trending` mengembalikan daftar kata kunci yang paling banyak dicari.
- [ ] Di frontend Svelte, input pencarian dilengkapi debounce (250ms) sehingga tidak membanjiri server dengan request berulang saat pengguna mengetik.
- [ ] Dropdown saran muncul di bawah search bar dengan navigasi keyboard (panah atas/bawah & Enter).
- [ ] Riwayat pencarian pengguna tersimpan secara lokal dan dapat dihapus dengan 1 klik ("Hapus Riwayat").
- [ ] Minimal 3 unit test untuk endpoint saran pencarian dan sanitasi karakter query khusus SQL.

---

## 📐 Langkah-Langkah Pengerjaan

### Langkah 1: Indeks Full-Text Search (SQLite FTS5)
**File:** `migrations/sqlite/023_create_search_fts.sql`

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS catalog_fts USING fts5(
    product_id UNINDEXED,
    name,
    description,
    category,
    content='catalog',
    content_rowid='rowid'
);

-- Triggers untuk sinkronisasi FTS otomatis saat catalog diubah
CREATE TRIGGER IF NOT EXISTS catalog_ai AFTER INSERT ON catalog BEGIN
  INSERT INTO catalog_fts(rowid, product_id, name, description, category) 
  VALUES (new.rowid, new.id, new.name, coalesce(new.description, ''), new.category);
END;

CREATE TRIGGER IF NOT EXISTS catalog_ad AFTER DELETE ON catalog BEGIN
  INSERT INTO catalog_fts(catalog_fts, rowid, product_id, name, description, category) 
  VALUES('delete', old.rowid, old.id, old.name, coalesce(old.description, ''), old.category);
END;

CREATE TRIGGER IF NOT EXISTS catalog_au AFTER UPDATE ON catalog BEGIN
  INSERT INTO catalog_fts(catalog_fts, rowid, product_id, name, description, category) 
  VALUES('delete', old.rowid, old.id, old.name, coalesce(old.description, ''), old.category);
  INSERT INTO catalog_fts(rowid, product_id, name, description, category) 
  VALUES (new.rowid, new.id, new.name, coalesce(new.description, ''), new.category);
END;

-- Tabel tracking kata kunci populer
CREATE TABLE IF NOT EXISTS search_analytics (
    id TEXT PRIMARY KEY NOT NULL,
    keyword TEXT NOT NULL UNIQUE,
    search_count INTEGER NOT NULL DEFAULT 1,
    last_searched_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### Langkah 2: Kontrak Interface & DTO
**File:** `crates/contracts/src/lib.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchSuggestionResult {
    pub query: String,
    pub product_suggestions: Vec<ProductSuggestionItem>,
    pub category_suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProductSuggestionItem {
    pub id: String,
    pub name: String,
    pub price_cents: i64,
    pub category: String,
    pub image_url: Option<String>,
}
```

### Langkah 3: REST API Handler
**File:** `crates/web/src/handlers/catalog.rs`
- Tambahkan handler `search_suggest` yang mengambil data dari FTS table dengan query prefix match (`name*`).
- Catat query pencarian ke tabel `search_analytics` secara asinkron tanpa memblokir request utama.

### Langkah 4: Tampilan Frontend Svelte 5
**File:** `frontend/src/App.svelte` & `frontend/src/lib/SearchDropdown.svelte`
- Buat komponen dropdown autocomplete mengambang (*floating suggestion card*).
- Tambahkan chip kata kunci pencarian populer di bawah search input.

### Langkah 5: Unit & Integration Tests
**File:** `crates/web/tests/search_suggest_test.rs`
- Uji autocomplete saran kata kunci sebagian (misal: ketik "sep" mengembalikan "Sepatu").
- Uji sanitasi simbol khusus FTS (seperti kutip, tanda bintang, boolean syntax) agar tidak menyebabkan crash SQL.
