use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};

use crate::error::ApiError;
use crate::state::AppState;
use program1_contracts::{BackupFileDto, DatabaseHealthDto, ErrorCode};

/// Trigger instant atomic SQLite database backup (Admin only)
#[utoipa::path(
    post,
    path = "/api/v1/admin/database/backup",
    responses(
        (status = 200, description = "Database backup created successfully", body = BackupFileDto),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "Database"
)]
pub async fn create_backup_handler(
    State(state): State<AppState>,
) -> Result<Json<BackupFileDto>, ApiError> {
    let backup = state.backup_contract.create_backup().await?;
    Ok(Json(backup))
}

/// List available database backup files (Admin only)
#[utoipa::path(
    get,
    path = "/api/v1/admin/database/backups",
    responses(
        (status = 200, description = "List of backup files", body = Vec<BackupFileDto>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "Database"
)]
pub async fn list_backups_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<BackupFileDto>>, ApiError> {
    let backups = state.backup_contract.list_backups().await?;
    Ok(Json(backups))
}

/// Download a database backup file (Admin only)
#[utoipa::path(
    get,
    path = "/api/v1/admin/database/backups/{filename}/download",
    params(
        ("filename" = String, Path, description = "Name of the backup file to download")
    ),
    responses(
        (status = 200, description = "Database backup binary stream"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Backup file not found")
    ),
    security(("bearer_auth" = [])),
    tag = "Database"
)]
pub async fn download_backup_handler(
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> Result<Response, ApiError> {
    let file_path = state.backup_contract.get_backup_path(&filename).await?;

    let bytes = tokio::fs::read(&file_path)
        .await
        .map_err(|e| ApiError::new(ErrorCode::ResourceNotFound, format!("Gagal membaca file backup: {}", e), StatusCode::NOT_FOUND))?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .header(header::CONTENT_LENGTH, bytes.len())
        .body(Body::from(bytes))
        .map_err(|e| ApiError::new(ErrorCode::InternalError, e.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(response)
}

/// Check database integrity health (Admin only)
#[utoipa::path(
    get,
    path = "/api/v1/admin/database/health",
    responses(
        (status = 200, description = "Database health and integrity status", body = DatabaseHealthDto),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "Database"
)]
pub async fn check_database_health_handler(
    State(state): State<AppState>,
) -> Result<Json<DatabaseHealthDto>, ApiError> {
    let health = state.backup_contract.check_health().await?;
    Ok(Json(health))
}
