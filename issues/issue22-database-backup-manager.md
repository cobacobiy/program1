# Issue 22: Automated Database Backup & Admin Recovery Manager (Phase 4 — #5)

> **Prioritas:** 🔴 CRITICAL  
> **Estimasi:** 2-3 hari  
> **Kesulitan:** ⭐⭐ Menengah  
> **Prerequisite:** Selesai Issue #3 (Database Persistence)  

---

## 📋 Deskripsi

Di lingkungan server produksi (Server 226 Linux), menjaga keutuhan data pelanggan, transaksi, dan katalog adalah prioritas tertinggi (*business continuity*). Kerusakan hard disk atau kesalahan manusia tanpa backup adalah bencana fatal.

Issue ini mengimplementasikan:
1. **Pencadangan Otomatis Terjadwal (Scheduled Automated Backups)**: Cron / background task tokio di backend yang mengeksekusi backup SQLite online vacuum / snapshot atau `pg_dump` secara harian.
2. **Kompresi & Rotasi Retensi**: Backup dikompresi menjadi format `.tar.gz` atau `.db.gz` dengan rotasi otomatis (menyimpan 7 backup harian terakhir dan 4 backup mingguan agar storage server tidak penuh).
3. **Admin Backup Manager di Admin Hub**: Admin dapat:
   - Melihat daftar file backup yang tersedia beserta ukuran dan tanggal pembuatan.
   - Mengunduh file backup langsung ke komputer lokal (*One-Click Download*).
   - Menjalankan pencadangan manual instan ("⚡ Backup Database Sekarang").
   - Melihat status kesehatan integritas database (`PRAGMA integrity_check`).

---

## 🎯 Acceptance Criteria

- [ ] Endpoint `POST /admin/database/backup` menghasilkan snapshot database baru di direktori volume aman (`/app/data/backups/`).
- [ ] Endpoint `GET /admin/database/backups` mengembalikan daftar file backup (nama file, tanggal, ukuran byte).
- [ ] Endpoint `GET /admin/database/backups/:filename/download` mengalirkan file backup yang terkompresi secara aman dengan verifikasi hak akses Super Admin.
- [ ] Background cron/task otomatis membersihkan backup lama yang melewati masa retensi (lebih dari 14 hari).
- [ ] Tab "💾 Database & Backup" tersedia di Admin Hub dengan visualisasi status integritas DB dan riwayat file backup.
- [ ] Minimal 3 unit test untuk: verifikasi pembuatan snapshot, otentikasi ketat unduh backup (menolak non-admin dan buyer), serta verifikasi integritas file output backup.

---

## 📐 Langkah-Langkah Pengerjaan

### Langkah 1: Modul Utilitas Backup di Core
**File:** `crates/core/src/backup.rs`

Implementasikan logika online backup SQLite yang aman dari data korup:
```rust
use std::path::{Path, PathBuf};
use chrono::Utc;
use sqlx::SqlitePool;

pub struct BackupManager {
    backup_dir: PathBuf,
}

impl BackupManager {
    pub fn new(backup_dir: impl Into<PathBuf>) -> Self {
        let dir = backup_dir.into();
        std::fs::create_dir_all(&dir).ok();
        Self { backup_dir: dir }
    }

    /// Menjalankan SQLite online VACUUM INTO untuk snapshot atomic tanpa downtime
    pub async fn create_sqlite_backup(&self, pool: &SqlitePool) -> Result<PathBuf, String> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("backup_program1_{}.db", timestamp);
        let target_path = self.backup_dir.join(&filename);

        let query = format!("VACUUM INTO '{}'", target_path.to_str().unwrap());
        sqlx::query(&query)
            .execute(pool)
            .await
            .map_err(|e| format!("Gagal mengeksekusi VACUUM backup: {}", e))?;

        Ok(target_path)
    }

    pub fn list_backups(&self) -> Vec<BackupMetadata> {
        // Baca file di backup_dir dan sort dari yang terbaru
    }

    pub fn prune_old_backups(&self, keep_days: u32) {
        // Hapus file backup yang umurnya lebih dari keep_days
    }
}
```

### Langkah 2: REST API Endpoints di Web
**File:** `crates/web/src/handlers/backup.rs`
- `POST /admin/database/backup` — Menjalankan backup dan mengembalikan metadata file yang terbentuk.
- `GET /admin/database/backups` — List semua file backup.
- `GET /admin/database/backups/:name/download` — Download file backup via stream axum `Body`.
- `GET /admin/database/health` — Menjalankan `PRAGMA integrity_check` dan mengembalikan status OK.

### Langkah 3: Tampilan Frontend Admin Hub
**File:** `frontend/src/lib/AdminHub.svelte`
- Tambahkan tab **"💾 Database & Backup"**.
- Tampilkan tombol utama **"⚡ Backup Database Sekarang"** dengan indikator loading.
- Tabel daftar file backup dengan tombol **"⬇️ Unduh File"**.

### Langkah 4: Unit & Integration Tests
**File:** `crates/web/tests/backup_test.rs`
- Uji endpoint backup menghasilkan file yang valid.
- Uji proteksi keamanan (pembeli/buyer mendapatkan status 403 Forbidden).
