use std::fs;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tracing::{info, warn};

use crate::database::DbPool;
use program1_contracts::{
    async_trait, BackupContract, BackupFileDto, ContractError, DatabaseHealthDto,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub filename: String,
    pub size_bytes: u64,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct BackupManager {
    backup_dir: PathBuf,
}

impl BackupManager {
    pub fn new(backup_dir: impl Into<PathBuf>) -> Self {
        let dir = backup_dir.into();
        if let Err(e) = fs::create_dir_all(&dir) {
            warn!("Could not pre-create backup directory {:?}: {}", dir, e);
        }
        Self { backup_dir: dir }
    }

    pub fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }

    /// Creates an atomic SQLite backup snapshot using VACUUM INTO
    pub async fn create_backup(&self, pool: &DbPool) -> Result<BackupMetadata, String> {
        fs::create_dir_all(&self.backup_dir)
            .map_err(|e| format!("Failed to create backup directory: {}", e))?;

        let abs_dir = fs::canonicalize(&self.backup_dir)
            .map_err(|e| format!("Failed to canonicalize backup directory: {}", e))?;

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("backup_program1_{}.db", timestamp);
        let target_path = abs_dir.join(&filename);

        let target_str = target_path
            .to_str()
            .ok_or_else(|| "Invalid UTF-8 in target backup path".to_string())?
            .replace('\'', "''");

        let vacuum_sql = format!("VACUUM INTO '{}'", target_str);
        sqlx::query(&vacuum_sql)
            .execute(pool)
            .await
            .map_err(|e| format!("Failed to execute VACUUM INTO: {}", e))?;

        if !target_path.exists() {
            let entries: Vec<String> = fs::read_dir(&abs_dir)
                .map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect())
                .unwrap_or_default();
            return Err(format!(
                "VACUUM INTO did not create file at {:?}. Dir entries: {:?}. SQL was: {}",
                target_path, entries, vacuum_sql
            ));
        }

        let metadata = fs::metadata(&target_path)
            .map_err(|e| format!("Failed to inspect created backup file {:?}: {}", target_path, e))?;

        info!(
            "Database backup created successfully: {:?} ({} bytes)",
            target_path,
            metadata.len()
        );

        Ok(BackupMetadata {
            filename,
            size_bytes: metadata.len(),
            created_at: Utc::now().to_rfc3339(),
        })
    }

    /// Lists all backup files in the backup directory, sorted newest first
    pub fn list_backups(&self) -> Vec<BackupMetadata> {
        let mut backups = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.backup_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let filename = entry.file_name().to_string_lossy().to_string();
                    if filename.starts_with("backup_program1_") && (filename.ends_with(".db") || filename.ends_with(".gz")) {
                        if let Ok(meta) = entry.metadata() {
                            let created_at = meta
                                .modified()
                                .ok()
                                .and_then(|t| {
                                    let dt: DateTime<Utc> = t.into();
                                    Some(dt.to_rfc3339())
                                })
                                .unwrap_or_else(|| Utc::now().to_rfc3339());

                            backups.push(BackupMetadata {
                                filename,
                                size_bytes: meta.len(),
                                created_at,
                            });
                        }
                    }
                }
            }
        }
        backups.sort_by(|a, b| b.filename.cmp(&a.filename));
        backups
    }

    /// Resolves and validates a backup file path to prevent directory traversal attacks
    pub fn get_backup_path(&self, filename: &str) -> Result<PathBuf, String> {
        if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
            return Err("Invalid backup filename".to_string());
        }
        if !filename.starts_with("backup_program1_") {
            return Err("Unrecognized backup file pattern".to_string());
        }
        let abs_dir = fs::canonicalize(&self.backup_dir)
            .unwrap_or_else(|_| self.backup_dir.clone());
        let path = abs_dir.join(filename);
        if !path.is_file() {
            return Err("Backup file does not exist".to_string());
        }
        Ok(path)
    }

    /// Prunes backup files older than keep_days
    pub fn prune_old_backups(&self, keep_days: u32) -> usize {
        let now = Utc::now();
        let max_age_secs = (keep_days as i64) * 86400;
        let mut deleted = 0;

        if let Ok(entries) = fs::read_dir(&self.backup_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(meta) = entry.metadata() {
                        if let Ok(mod_time) = meta.modified() {
                            let mod_dt: DateTime<Utc> = mod_time.into();
                            let age_secs = now.signed_duration_since(mod_dt).num_seconds();
                            if age_secs > max_age_secs {
                                if fs::remove_file(&path).is_ok() {
                                    deleted += 1;
                                    info!("Pruned old backup file: {:?}", path);
                                }
                            }
                        }
                    }
                }
            }
        }
        deleted
    }

    /// Runs SQLite PRAGMA integrity_check to verify database health
    pub async fn verify_db_integrity(&self, pool: &DbPool) -> Result<String, String> {
        let row = sqlx::query("PRAGMA integrity_check;")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("Database integrity query error: {}", e))?;

        let status: String = row.try_get(0).unwrap_or_else(|_| "error".to_string());
        Ok(status)
    }
}

#[derive(Clone)]
pub struct BackupService {
    pool: DbPool,
    manager: BackupManager,
}

impl BackupService {
    pub fn new(pool: DbPool, backup_dir: impl Into<PathBuf>) -> Self {
        Self {
            pool,
            manager: BackupManager::new(backup_dir),
        }
    }

    pub fn manager(&self) -> &BackupManager {
        &self.manager
    }
}

#[async_trait]
impl BackupContract for BackupService {
    async fn create_backup(&self) -> Result<BackupFileDto, ContractError> {
        let meta = self
            .manager
            .create_backup(&self.pool)
            .await
            .map_err(ContractError::Internal)?;

        Ok(BackupFileDto {
            filename: meta.filename,
            size_bytes: meta.size_bytes,
            created_at: meta.created_at,
        })
    }

    async fn list_backups(&self) -> Result<Vec<BackupFileDto>, ContractError> {
        let metas = self.manager.list_backups();
        Ok(metas
            .into_iter()
            .map(|m| BackupFileDto {
                filename: m.filename,
                size_bytes: m.size_bytes,
                created_at: m.created_at,
            })
            .collect())
    }

    async fn get_backup_path(&self, filename: &str) -> Result<PathBuf, ContractError> {
        self.manager
            .get_backup_path(filename)
            .map_err(ContractError::ValidationError)
    }

    async fn check_health(&self) -> Result<DatabaseHealthDto, ContractError> {
        let integrity = self
            .manager
            .verify_db_integrity(&self.pool)
            .await
            .map_err(ContractError::Internal)?;

        Ok(DatabaseHealthDto {
            status: if integrity == "ok" {
                "healthy".to_string()
            } else {
                "degraded".to_string()
            },
            engine: "sqlite".to_string(),
            integrity,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_backup_manager_direct() {
        let db_file = format!("./target/test_direct_db_{}.db", rand::random::<u64>());
        let db_url = format!("sqlite:{}", db_file);
        let pool = crate::init_database(&db_url).await.unwrap();
        let mgr = BackupManager::new("./target/test_direct_backup");
        let res = mgr.create_backup(&pool).await;
        assert!(res.is_ok(), "Direct backup failed: {:?}", res);
        let list = mgr.list_backups();
        assert!(!list.is_empty());
        let health = mgr.verify_db_integrity(&pool).await.unwrap();
        assert_eq!(health, "ok");
        let _ = std::fs::remove_file(db_file);
    }
}

