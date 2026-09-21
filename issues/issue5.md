# Issue #5: Server Tidak Memiliki Graceful Shutdown — Data Loss Risiko

## Severity: 🟠 MEDIUM (Reliability / Data Integrity)

## Deskripsi
`main.rs` menggunakan `axum::serve(listener, app).await.unwrap()` tanpa signal handler untuk graceful shutdown. Ketika server menerima SIGTERM (dari Docker stop, deployment, atau Ctrl+C):
- Request yang sedang diproses akan di-terminate secara paksa
- Database transaction yang belum commit bisa corrupt
- Backup yang sedang berjalan bisa menghasilkan file corrupt
- WebSocket/SSE connections terputus tanpa notifikasi

Docker mengirim SIGTERM dan menunggu 10 detik sebelum SIGKILL, tapi tanpa graceful shutdown handler, server langsung crash pada SIGTERM.

## File yang Terdampak
- `crates/web/src/main.rs` — baris 206-207

## Bukti Masalah
```rust
// main.rs:206-207
let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
axum::serve(listener, app).await.unwrap();
// ❌ Tidak ada graceful shutdown signal handling
```

## Langkah Perbaikan

### Step 1: Tambahkan shutdown signal handler
Buka file: `crates/web/src/main.rs`

Tambahkan function baru sebelum `main()`:
```rust
/// Graceful shutdown signal handler (SIGTERM for Docker, Ctrl+C for dev)
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { tracing::info!("Received Ctrl+C, initiating graceful shutdown..."); },
        _ = terminate => { tracing::info!("Received SIGTERM, initiating graceful shutdown..."); },
    }
}
```

### Step 2: Update server startup untuk menggunakan graceful shutdown
Ganti baris 206-207:
```rust
// Sebelum:
let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
axum::serve(listener, app).await.unwrap();
```

Menjadi:
```rust
let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();

tracing::info!("Program1 server shut down gracefully.");
```

### Step 3: Verifikasi
```bash
cargo check --workspace
cargo test --workspace
```

### Step 4: Test Manual
1. Jalankan server: `cargo run -p program1-web`
2. Tekan Ctrl+C
3. Pastikan log menampilkan "Received Ctrl+C, initiating graceful shutdown..." dan "Program1 server shut down gracefully."
4. Pastikan tidak ada error/panic

## Estimasi Waktu: 15 menit
## Kompleksitas: Rendah
