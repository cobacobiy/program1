# Issue 17: Production Build Integration, Dockerfile Multi-stage (Bun + Rust) & Axum Static Serving

> **Prioritas:** 🔴 CRITICAL  
> **Estimasi:** 3-4 hari  
> **Kesulitan:** ⭐⭐⭐ Menengah  
> **Prerequisite:** Issue 11 s/d Issue 16

---

## 📋 Deskripsi

Sesuai aturan arsitektur utama di `AGENTS.md`:
1. **Single Binary Deployment:** Seluruh aplikasi dideploy sebagai **satu binary mandiri** (atau container Docker mandiri) tanpa perlu menginstal runtime Node.js/Bun di server produksi.
2. **Multiplatform Docker:** Container harus dapat dibangun dan dijalankan di Linux Server 226 maupun Windows Staging.

Di Issue ini, kita akan:
1. Mengintegrasikan hasil kompilasi Svelte (`frontend/dist/`) agar disajikan secara langsung oleh backend Rust Axum menggunakan `tower_http::services::ServeDir` dengan **SPA Fallback Routing**.
2. Memperbarui `Dockerfile` menjadi **Multi-stage Build** 3 tahap:
   - **Stage 1 (Frontend Builder):** Menggunakan `oven/bun:1` untuk meng-compile komponen Svelte menjadi aset statis murni (HTML, CSS, JS terkompresi).
   - **Stage 2 (Rust Builder):** Mengompilasi binary Rust `program1-web`.
   - **Stage 3 (Runtime):** Container Debian/Alpine ultra-ramping dan aman untuk produksi tanpa Node/Bun.
3. Memperbarui pipeline CI/CD GitHub Actions agar build otomatis berjalan lancar.

---

## 🎯 Acceptance Criteria

- [ ] Command `bun run build` menghasilkan aset statis di `crates/web/static/dist/`.
- [ ] Server Axum di `crates/web/src/routes.rs` menyajikan file dari folder `dist/` dan memiliki SPA fallback (refresh halaman pada URL client-side tidak menghasilkan error 404).
- [ ] `Dockerfile` menggunakan 3-stage multi-stage build yang berhasil di-build tanpa error.
- [ ] Image Docker produksi tetap berukuran kecil (< 150MB) dan tidak mengandung `node_modules` atau compiler frontend.
- [ ] `docker compose config --quiet` valid tanpa peringatan.
- [ ] Seluruh unit test tetap 100% lulus: `cargo test --workspace`.

---

## 📐 Langkah-Langkah Pengerjaan

### Langkah 1: Atur Output Build di `frontend/vite.config.ts`

Pastikan output build diarahkan langsung ke folder statis backend:

```typescript
// frontend/vite.config.ts
export default defineConfig({
  // ... plugins & resolve
  build: {
    outDir: '../crates/web/static/dist',
    emptyOutDir: true,
  },
});
```

---

### Langkah 2: Konfigurasi SPA Fallback Routing di Rust Axum

Buka file `crates/web/src/routes.rs`.  
Cari bagian `static_routes` (sekitar baris 380-395), lalu ubah menjadi penyajian aset `dist` dengan fallback ke `index.html`:

```rust
use tower_http::services::{ServeDir, ServeFile};

// Periksa apakah folder dist dari Svelte sudah ter-build
let dist_dir = "crates/web/static/dist";
let fallback_file = format!("{}/index.html", dist_dir);

let static_service = if std::path::Path::new(&fallback_file).exists() {
    // Mode Modern: Svelte SPA
    ServeDir::new(dist_dir).fallback(ServeFile::new(fallback_file))
} else {
    // Mode Fallback: Legacy static files jika Svelte belum di-build
    ServeDir::new("crates/web/static")
};

let static_routes = Router::new()
    .nest_service("/assets", ServeDir::new("crates/web/static/dist/assets"))
    .nest_service("/uploads", ServeDir::new("data/uploads"))
    .fallback_service(static_service);
```

---

### Langkah 3: Perbarui `Dockerfile` Menjadi Multi-stage Build (Bun + Rust)

Buka file `Dockerfile` di root repository `program1`, perbarui isinya menjadi:

```dockerfile
# ============================================
# Stage 1: Build Frontend (Svelte 5 + Bun)
# ============================================
FROM oven/bun:1-alpine AS frontend-builder
WORKDIR /app/frontend

COPY frontend/package.json frontend/bun.lockb* frontend/bun.lock* ./
RUN bun install --frozen-lockfile || bun install

COPY frontend/ ./
RUN bun run build

# ============================================
# Stage 2: Build Backend Binary (Rust Axum)
# ============================================
FROM rust:1.80-slim-bookworm AS rust-builder
WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    sqlite3 \
    libsqlite3-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo workspace configuration
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY migrations/ ./migrations/

# Copy compiled Svelte frontend assets from Stage 1
COPY --from=frontend-builder /app/crates/web/static/dist ./crates/web/static/dist

# Build release binary
ENV SQLX_OFFLINE=true
RUN cargo build --release -p program1-web

# ============================================
# Stage 3: Minimal Production Runtime
# ============================================
FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libsqlite3-0 \
    curl \
    tzdata \
    && rm -rf /var/lib/apt/lists/*

# Copy compiled Rust binary
COPY --from=rust-builder /app/target/release/program1-web /app/program1-web

# Copy static frontend & migrations
COPY --from=frontend-builder /app/crates/web/static/dist /app/crates/web/static/dist
COPY migrations/ /app/migrations/
COPY .env.example /app/.env.example

# Create uploads directory
RUN mkdir -p /app/data/uploads

ENV APP_HOST=0.0.0.0
ENV APP_PORT=8080
EXPOSE 8080

HEALTHCHECK --interval=15s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -fsS http://localhost:8080/health || exit 1

ENTRYPOINT ["/app/program1-web"]
```

---

### Langkah 4: Perbarui CI/CD Workflow

Buka file `.github/workflows/ci-cd.yml`. Pada job `build-ghcr`, Docker Buildx akan otomatis mengeksekusi stage `frontend-builder` (Bun) dan `rust-builder` tanpa memerlukan konfigurasi tambahan.

Pastikan branch `main` tidak mengabaikan perubahan di folder `frontend/**`:
Hapus `frontend/**` jika ada di `paths-ignore`.

---

## 🧪 Pengujian & Verifikasi

1. **Build Frontend Lokal:**
   ```bash
   cd frontend
   bun run build
   cd ..
   ```
   Pastikan folder `crates/web/static/dist` terbuat dan berisi file `index.html` serta folder `assets/`.

2. **Jalankan Rust Binary Lokal:**
   ```bash
   cargo run -p program1-web
   ```
   Buka browser ke `http://localhost:8080`.
   - Pastikan aplikasi Svelte termuat sempurna langsung dari server Axum tanpa Vite dev server!
   - Coba lakukan refresh di sub-route untuk membuktikan SPA fallback bekerja tanpa 404.

3. **Jalankan Seluruh Unit Test Workspace (Wajib AGENTS.md):**
   ```bash
   cargo test --workspace
   ```
   Harus berstatus **100% lulus (ALL PASSED)**.

4. **Uji Validasi Docker:**
   ```bash
   docker compose config --quiet
   docker compose build
   ```

---

## ⚠️ Hal Penting & Gotchas

- **Ukuran Image Produksi:** Dengan multi-stage build, container produksi hanya berisi binary Rust dan aset statis (~60-120 MB), tidak membawa compiler Bun ataupun Cargo!
- **SPA Fallback Routing:** Pastikan rute API (`/api/*`, `/health`, `/auth/*`, `/uploads/*`) diprioritaskan sebelum fallback SPA agar endpoint API tidak mengembalikan file HTML secara tidak sengaja.
