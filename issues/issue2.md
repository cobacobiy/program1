# Issue #2: Rate Limiter Memory Leak — `cleanup_stale_entries()` Tidak Pernah Dipanggil

## Severity: 🟠 MEDIUM (Memory Leak Lambat)

## Deskripsi
`IpRateLimiter` di `rate_limit.rs` menyimpan semua timestamp request per IP/route di `HashMap<(String, String), Vec<Instant>>`. Method `cleanup_stale_entries()` sudah ditulis untuk membersihkan entry yang kosong, tetapi **tidak pernah dipanggil dari mana pun** di seluruh codebase.

Ini berarti:
- Setiap IP unik yang pernah mengirim request akan tetap tersimpan di memory selamanya
- Di production dengan traffic tinggi, `HashMap` ini akan terus membesar tanpa batas
- Memory usage server akan naik perlahan dan tidak pernah turun (memory leak)

## File yang Terdampak
- `crates/web/src/rate_limit.rs` — method `cleanup_stale_entries()` (baris 90-93)
- `crates/web/src/main.rs` — harus ditambahkan background task

## Bukti Masalah
```bash
# Cari penggunaan cleanup_stale_entries di seluruh project
grep -rn "cleanup_stale_entries" crates/
```
Hasil: Hanya ditemukan di **definisi** (baris 90), tidak ada pemanggilan.

## Langkah Perbaikan

### Step 1: Tambahkan background cleanup task di `main.rs`
Buka file: `crates/web/src/main.rs`

Tambahkan setelah baris `let app = create_app(state);` (sebelum bind listener), kira-kira setelah baris 201:

```rust
    // Spawn periodic rate limiter cleanup task (every 5 minutes)
    let cleanup_limiter = state.rate_limiter.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            cleanup_limiter.cleanup_stale_entries().await;
            tracing::debug!("Rate limiter stale entries cleaned up");
        }
    });
```

**Catatan**: Variable `state` sudah di-move ke `create_app(state)` di baris 201. Kamu perlu meng-clone `rate_limiter` **sebelum** `create_app` dipanggil. Jadi letakkan clone di atas baris `let app = create_app(state);`:

```rust
    // Clone rate_limiter sebelum state di-move ke create_app
    let cleanup_limiter = state.rate_limiter.clone();

    let app = create_app(state);

    // Spawn periodic rate limiter cleanup task (every 5 minutes)
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            cleanup_limiter.cleanup_stale_entries().await;
            tracing::debug!("Rate limiter stale entries cleaned up");
        }
    });
```

### Step 2: Verifikasi
```bash
cargo check --workspace
cargo test --workspace
```

### Step 3: Cek log setelah 5 menit berjalan
Jalankan server dengan `RUST_LOG=debug` dan pastikan log `"Rate limiter stale entries cleaned up"` muncul setiap 5 menit.

## Estimasi Waktu: 15 menit
## Kompleksitas: Rendah
