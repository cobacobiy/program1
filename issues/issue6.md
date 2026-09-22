# Issue #6: Backup Pruning Tidak Pernah Dijalankan Otomatis

## Severity: 🟡 LOW-MEDIUM (Disk Space)

## Deskripsi
`BackupManager` memiliki method `prune_old_backups(keep_days)` yang sudah diimplementasikan untuk menghapus backup file yang lebih tua dari X hari. Namun:

1. Method ini **tidak pernah dipanggil secara otomatis** (tidak ada scheduled task/cron)
2. **Tidak ada endpoint API** untuk trigger manual pruning
3. Backup file di `./data/backups/` akan terus menumpuk tanpa batas

Setiap file backup SQLite bisa berukuran puluhan MB. Tanpa auto-pruning, disk di production server bisa penuh seiring waktu.

## File yang Terdampak
- `crates/core/src/backup.rs` — method `prune_old_backups()` (baris 138-163, sudah ada tapi tidak dipakai)
- `crates/web/src/main.rs` — perlu ditambahkan scheduled task
- *Opsional*: `crates/web/src/handlers/backup.rs` — bisa ditambahkan endpoint manual

## Bukti Masalah
```bash
# Cari penggunaan prune_old_backups di seluruh codebase
grep -rn "prune_old_backups" crates/
```
Hasil: Hanya ditemukan di **definisi** function, tidak ada pemanggilan.

## Langkah Perbaikan

### Opsi A: Auto-Prune Setelah Setiap Backup (Paling Sederhana)
Buka file: `crates/core/src/backup.rs`

Di method `create_backup()` pada `BackupService` impl (baris 198-210), tambahkan auto-prune setelah backup berhasil:

```rust
#[async_trait]
impl BackupContract for BackupService {
    async fn create_backup(&self) -> Result<BackupFileDto, ContractError> {
        let meta = self
            .manager
            .create_backup(&self.pool)
            .await
            .map_err(ContractError::Internal)?;

        // Auto-prune backups older than 30 days after successful backup
        let pruned = self.manager.prune_old_backups(30);
        if pruned > 0 {
            tracing::info!("Auto-pruned {} old backup file(s)", pruned);
        }

        Ok(BackupFileDto {
            filename: meta.filename,
            size_bytes: meta.size_bytes,
            created_at: meta.created_at,
        })
    }
    // ... rest of impl
}
```

### Opsi B: Background Scheduled Task (Lebih Robust)
Tambahkan di `main.rs` setelah inisialisasi `backup_service`:

```rust
// Spawn daily backup pruning task (runs every 24 hours)
let prune_manager = backup_service.manager().clone();
tokio::spawn(async move {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(86400));
    loop {
        interval.tick().await;
        let pruned = prune_manager.prune_old_backups(30);
        if pruned > 0 {
            tracing::info!("Scheduled cleanup: pruned {} old backup file(s)", pruned);
        }
    }
});
```

**Catatan**: `BackupManager` perlu di-derive `Clone` jika belum (sudah ada `#[derive(Debug, Clone)]` di baris 20-21, jadi aman).

### Step 2: Verifikasi
```bash
cargo check --workspace
cargo test --workspace
```

## Konfigurasi Retention (Opsional, Future Improvement)
Bisa ditambahkan env var `BACKUP_RETENTION_DAYS` di `.env.example`:
```
BACKUP_RETENTION_DAYS=30
```
Tapi untuk MVP, hardcode 30 hari sudah cukup.

## Estimasi Waktu: 15-30 menit
## Kompleksitas: Rendah
