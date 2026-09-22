# Issue #4: `expires_in` di Auth Token Response Hardcoded 86400, Tidak Mengikuti Config

## Severity: 🟡 LOW-MEDIUM (Inconsistency Bug)

## Deskripsi
Ketika user berhasil login (seller maupun buyer), response field `expires_in` selalu hardcoded `86400` detik (24 jam). Padahal actual JWT expiry dikonfigurasi melalui environment variable `JWT_EXPIRY_HOURS` (default 24 jam, bisa diubah). Jika admin mengubah `JWT_EXPIRY_HOURS` ke 8 jam, frontend tetap menampilkan token valid 24 jam — menyebabkan confusion dan bug session management.

## File yang Terdampak
- `crates/web/src/handlers/auth.rs` — baris 54 (`expires_in: 86400`)
- `crates/modules/buyer/src/lib.rs` — baris 678, 765, 891 (`expires_in: 86400`)

## Bukti Masalah
```rust
// handlers/auth.rs:54
let token_resp = AuthTokenResponse {
    access_token: token,
    token_type: "Bearer".to_string(),
    expires_in: 86400,  // ❌ Hardcoded! Seharusnya dari config
    user,
};
```

Sementara config di `core/config.rs`:
```rust
jwt_expiry_hours: env_or("JWT_EXPIRY_HOURS", "24").parse().unwrap_or(24),
```

## Langkah Perbaikan

### Step 1: Tambahkan `jwt_expiry_hours` ke AppState
Buka file: `crates/web/src/state.rs`

Tambahkan field baru di struct `AppState`:
```rust
pub jwt_expiry_hours: u64,
```

### Step 2: Pass config value saat membuat AppState
Buka file: `crates/web/src/main.rs`

Di struct initialization `AppState { ... }` (sekitar baris 173), tambahkan:
```rust
jwt_expiry_hours: config.jwt_expiry_hours,
```

### Step 3: Update `auth.rs` handler
Buka file: `crates/web/src/handlers/auth.rs`

Ganti baris 54:
```rust
// Sebelum:
expires_in: 86400,

// Sesudah:
expires_in: state.jwt_expiry_hours * 3600,
```

**Catatan:** `jwt_expiry_hours` dikalikan 3600 karena `expires_in` menggunakan satuan detik.

### Step 4: Update buyer module
Buka file: `crates/modules/buyer/src/lib.rs`

Untuk buyer module, perlu ditambahkan `jwt_expiry_hours` ke konfigurasi `BuyerModule` atau dipass dari luar. Pendekatan paling sederhana:

Di struct `BuyerModuleConfig`, tambahkan field:
```rust
pub jwt_expiry_hours: u64,
```

Lalu update 3 lokasi yang menggunakan `expires_in: 86400` (baris 678, 765, 891):
```rust
// Ganti setiap kemunculan
expires_in: self.config.jwt_expiry_hours * 3600,
```

Dan update `main.rs` saat membuat `buyer_config`:
```rust
let buyer_config = program1_module_buyer::BuyerModuleConfig {
    otp_expiry_seconds: config.otp_expiry_seconds as u64,
    otp_max_attempts: config.otp_max_attempts as u32,
    otp_resend_cooldown_seconds: config.otp_resend_cooldown_seconds as u64,
    jwt_expiry_hours: config.jwt_expiry_hours,  // ← tambahkan ini
};
```

### Step 5: Verifikasi
```bash
cargo check --workspace
cargo test --workspace
```

### Step 6: Test Manual
1. Set `JWT_EXPIRY_HOURS=1` di `.env`
2. Login via API
3. Cek response `expires_in` harus `3600` (bukan `86400`)

## Estimasi Waktu: 30 menit
## Kompleksitas: Rendah
