# Issue #3: Audit Log Tidak Mencatat IP Address Client

## Severity: 🟠 MEDIUM (Security & Compliance Gap)

## Deskripsi
Semua audit log entry di seluruh handler selalu meng-set `ip_address: None`. Padahal `AuditLogEntry` sudah menyediakan field `ip_address: Option<String>`, dan system sudah punya fungsi `extract_client_ip()` di `rate_limit.rs` yang siap dipakai.

Tanpa IP address di audit log:
- Tidak bisa tracing aktivitas mencurigakan ke sumber IP tertentu
- Investigasi security incident (brute-force, unauthorized access) jadi sulit
- Tidak comply dengan best practice audit trail

## File yang Terdampak (30+ lokasi)
- `crates/web/src/handlers/auth.rs` — 3 tempat (login success, login failed, register)
- `crates/web/src/handlers/user.rs` — 5 tempat
- `crates/web/src/handlers/order.rs` — 6 tempat
- `crates/web/src/handlers/catalog.rs` — 4 tempat
- `crates/web/src/handlers/inventory.rs` — 5 tempat
- `crates/web/src/handlers/category.rs` — 3 tempat
- `crates/web/src/handlers/coupon.rs` — 3 tempat
- `crates/web/src/handlers/channel.rs` — 1 tempat

## Langkah Perbaikan

### Step 1: Pahami Pola yang Dipakai
Setiap handler menerima `Request` atau menggunakan extractor Axum. Kita perlu menambahkan cara untuk mendapatkan client IP di setiap handler yang menulis audit log.

Ada 2 pendekatan:

**Pendekatan A (Recommended — via middleware injection):**
Buat middleware yang menginject `ClientIp(String)` ke request extensions, lalu extract di handler.

Tambahkan di `middleware.rs`:
```rust
/// Extension for client IP address
#[derive(Debug, Clone)]
pub struct ClientIp(pub String);
```

Lalu di `request_id_middleware`, tambahkan sebelum `next.run(req)`:
```rust
let client_ip = crate::rate_limit::extract_client_ip(&req);
req.extensions_mut().insert(ClientIp(client_ip));
```

**Pendekatan B (Simpler — per handler):**
Tambahkan parameter `ConnectInfo` di handler yang perlu audit, tapi ini lebih invasif.

### Step 2: Update setiap handler untuk menggunakan `ClientIp`
Di setiap handler yang menulis audit log, tambahkan extractor:
```rust
use crate::middleware::ClientIp;

pub async fn login_handler(
    State(state): State<AppState>,
    Extension(client_ip): Extension<ClientIp>,  // ← tambahkan ini
    ValidatedJson(payload): ValidatedJson<LoginRequest>,
) -> Result<Json<AuthTokenResponse>, ApiError> {
    // ... di AuditLogEntry, ganti:
    //   ip_address: None,
    // menjadi:
    //   ip_address: Some(client_ip.0.clone()),
}
```

### Step 3: Update semua `ip_address: None` menjadi `ip_address: Some(client_ip.0.clone())`
Gunakan pencarian berikut untuk menemukan semua lokasi:
```bash
grep -rn "ip_address: None" crates/web/src/handlers/
```

Ubah setiap kemunculan dari:
```rust
ip_address: None,
```
Menjadi:
```rust
ip_address: Some(client_ip.0.clone()),
```

Dan pastikan setiap handler yang terdampak sudah menambahkan `Extension(client_ip): Extension<ClientIp>` di function signature.

### Step 4: Verifikasi
```bash
cargo check --workspace
cargo test --workspace
```

## Daftar File dan Baris yang Perlu Diubah
| File | Baris | Konteks |
|------|-------|---------|
| `handlers/auth.rs` | 47, 71, 113 | login success, login failed, register |
| `handlers/user.rs` | 72, 119, 165, 208, 276 | CRUD user, break-glass |
| `handlers/order.rs` | 203, 258, 328, 389, 475, 554 | order status, tracking |
| `handlers/catalog.rs` | 123, 222, 278, 320 | CRUD catalog |
| `handlers/inventory.rs` | 149, 233, 293, 353, 466 | stock updates |
| `handlers/category.rs` | 86, 129, 170 | CRUD category |
| `handlers/coupon.rs` | 80, 130, 171 | CRUD coupon |
| `handlers/channel.rs` | 78 | channel sync |

## Estimasi Waktu: 1-2 jam
## Kompleksitas: Sedang (banyak file tapi perubahan repetitif)
