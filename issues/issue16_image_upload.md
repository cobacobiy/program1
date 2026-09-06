# Issue #16 — Product Image Upload & File Storage

> **Prioritas**: 🟡 HIGH
> **Estimasi**: 2-3 hari
> **Depends On**: —
> **Skill Level**: Junior-Mid Rust Developer

---

## 🔍 Masalah Saat Ini

Product `image_url` saat ini hanya menyimpan **URL string external** (contoh: `https://placehold.co/400`). Tidak ada:
- ❌ Fitur upload gambar dari admin panel
- ❌ File storage service (local atau S3-compatible)
- ❌ Validasi format & ukuran gambar
- ❌ Thumbnail/resize otomatis

Semua gambar produk yang ditampilkan di storefront bergantung pada URL placeholder external.

---

## ✅ Acceptance Criteria

### Step 1: Buat Upload Endpoint

| Method | Path | Auth | Deskripsi |
|--------|------|------|-----------|
| `POST` | `/api/v1/uploads/images` | Seller JWT | Upload gambar (multipart/form-data) |
| `GET` | `/uploads/:filename` | Public | Serve gambar yang sudah di-upload |

### Step 2: Tambahkan Dependencies

Di `crates/web/Cargo.toml`:
```toml
axum = { workspace = true, features = ["macros", "multipart"] }
```

### Step 3: Buat Upload Handler di `crates/web/src/handlers/upload.rs`

```rust
use axum::extract::Multipart;

pub async fn upload_image(
    _claims: JwtClaims,        // Seller auth required
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, ApiError> {
    // 1. Extract file dari multipart field
    // 2. Validasi: hanya accept image/jpeg, image/png, image/webp
    // 3. Validasi: max 5MB
    // 4. Generate unique filename: {uuid}.{ext}
    // 5. Simpan ke ./data/uploads/{filename}
    // 6. Return URL: /uploads/{filename}
}
```

**Response:**
```json
{
  "url": "/uploads/550e8400-e29b-41d4-a716-446655440000.jpg",
  "filename": "550e8400-e29b-41d4-a716-446655440000.jpg",
  "size_bytes": 245678
}
```

### Step 4: Serve Static Uploads

Di `crates/web/src/routes.rs`, tambahkan:
```rust
.nest_service("/uploads", ServeDir::new("data/uploads"))
```

### Step 5: Buat Directory `data/uploads/` di Dockerfile

```dockerfile
RUN mkdir -p /app/data/uploads
```

Dan di `docker-compose.yml`:
```yaml
volumes:
  - program1_uploads:/app/data/uploads
```

### Step 6: Frontend Integration (Admin Panel)

Di admin panel catalog form (`app.js`):
1. Tambahkan `<input type="file" accept="image/*">` di form create/edit product
2. Upload gambar dulu via `POST /api/v1/uploads/images`
3. Setelah berhasil, gunakan URL response sebagai `image_url` produk

```javascript
async function uploadProductImage(file) {
    const formData = new FormData();
    formData.append('image', file);
    
    const res = await fetch('/api/v1/uploads/images', {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${token}` },
        body: formData
    });
    const data = await res.json();
    return data.url;  // "/uploads/xxx.jpg"
}
```

### Step 7: Validasi & Security

- **Content-Type validation**: Cek magic bytes file, bukan hanya extension
- **Max file size**: 5 MB (atur di route layer: `DefaultBodyLimit::max(5 * 1024 * 1024)`)
- **Filename sanitization**: Selalu generate UUID baru, jangan pakai nama file asli
- **Rate limit**: 10 uploads per menit per IP

### Step 8: Unit Tests

1. Test upload valid JPEG → returns URL
2. Test upload file > 5MB → error `PayloadTooLarge`
3. Test upload non-image file (.txt, .exe) → error `ValidationFailed`
4. Test serve uploaded file → GET returns 200 with correct content-type
5. Test upload tanpa auth → 401

---

## 📁 File Yang Harus Dibuat/Diubah

| Action | File |
|--------|------|
| **CREATE** | `crates/web/src/handlers/upload.rs` |
| **MODIFY** | `crates/web/src/handlers/mod.rs` |
| **MODIFY** | `crates/web/src/routes.rs` (add upload route + /uploads serve) |
| **MODIFY** | `crates/web/Cargo.toml` (add multipart feature) |
| **MODIFY** | `Dockerfile` (mkdir data/uploads) |
| **MODIFY** | `docker-compose.yml` (add volume) |
| **MODIFY** | `crates/web/static/app.js` (admin image upload UI) |

---

## ⚠️ Catatan Penting

- Simpan file di **local disk** dulu (`./data/uploads/`), bukan S3. S3 bisa ditambah nanti.
- Volume mount di docker-compose supaya data persist antar container restart.
- JANGAN serve file dengan nama asli user — selalu rename ke UUID.
- Pastikan `cargo test --workspace` pass sebelum push.
