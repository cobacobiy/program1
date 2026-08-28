use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use tracing::info;

pub type DbPool = Pool<Sqlite>;

/// Initialize the database pool and run embedded migrations
pub async fn init_database(database_url: &str) -> Result<DbPool, sqlx::Error> {
    if database_url.starts_with("sqlite:") {
        // Parse sqlite path to ensure directory exists if file-based
        let clean_path = database_url
            .trim_start_matches("sqlite://")
            .trim_start_matches("sqlite:")
            .split('?')
            .next()
            .unwrap_or("");

        if !clean_path.is_empty() && clean_path != ":memory:" {
            if let Some(parent) = Path::new(clean_path).parent() {
                if !parent.as_os_str().is_empty() {
                    let _ = std::fs::create_dir_all(parent);
                }
            }
        }

        let connect_options = SqliteConnectOptions::from_str(database_url)?
            .create_if_missing(true)
            .busy_timeout(Duration::from_secs(5));

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .after_connect(|conn, _meta| {
                Box::pin(async move {
                    sqlx::query("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect_with(connect_options)
            .await?;

        info!("Running database migrations for SQLite...");
        sqlx::migrate!("../../migrations/sqlite")
            .run(&pool)
            .await?;

        info!("Database migrations executed successfully.");
        Ok(pool)
    } else {
        // Default to in-memory sqlite if not matching
        let connect_options = SqliteConnectOptions::from_str("sqlite::memory:")?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(connect_options)
            .await?;

        sqlx::migrate!("../../migrations/sqlite")
            .run(&pool)
            .await?;

        Ok(pool)
    }
}
