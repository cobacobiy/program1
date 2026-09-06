use axum::{
    extract::{Extension, Multipart},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::ApiError;
use program1_contracts::{ErrorCode, JwtClaims};

pub const MAX_FILE_SIZE: usize = 5 * 1024 * 1024; // 5 MB

/// Response payload for successfully uploaded image file
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UploadResponse {
    pub url: String,
    pub filename: String,
    pub size_bytes: usize,
}

/// Detect and validate image extension from file magic bytes
pub fn detect_image_extension(bytes: &[u8]) -> Result<&'static str, ApiError> {
    // JPEG magic bytes: FF D8 FF
    if bytes.len() >= 3 && &bytes[0..3] == [0xFF, 0xD8, 0xFF] {
        return Ok("jpg");
    }

    // PNG magic bytes: 89 50 4E 47 0D 0A 1A 0A
    if bytes.len() >= 8 && &bytes[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
        return Ok("png");
    }

    // WebP magic bytes: RIFF....WEBP
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Ok("webp");
    }

    Err(ApiError::new(
        ErrorCode::ValidationFailed,
        "Format file tidak didukung atau file bukan gambar yang valid. Hanya file JPEG, PNG, dan WebP yang diizinkan.",
        StatusCode::BAD_REQUEST,
    ))
}

/// Upload an image file (multipart/form-data) — Seller auth required
#[utoipa::path(
    post,
    path = "/api/v1/uploads/images",
    responses(
        (status = 200, description = "Image successfully uploaded", body = UploadResponse),
        (status = 400, description = "Validation failed / unsupported image format", body = ApiError),
        (status = 401, description = "Unauthorized - Valid Seller JWT required"),
        (status = 413, description = "Payload too large (> 5MB)", body = ApiError),
        (status = 429, description = "Rate limit exceeded", body = ApiError)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Uploads"
)]
pub async fn upload_image_handler(
    Extension(_claims): Extension<JwtClaims>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, ApiError> {
    let mut file_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        let status = e.status();
        let code = if status == StatusCode::PAYLOAD_TOO_LARGE {
            ErrorCode::PayloadTooLarge
        } else {
            ErrorCode::ValidationFailed
        };
        ApiError::new(
            code,
            format!("Gagal membaca payload multipart: {}", e),
            status,
        )
    })? {
        let field_name = field.name().unwrap_or_default().to_string();
        if field_name == "image" || field_name == "file" {
            let data = field.bytes().await.map_err(|e| {
                let status = e.status();
                let code = if status == StatusCode::PAYLOAD_TOO_LARGE {
                    ErrorCode::PayloadTooLarge
                } else {
                    ErrorCode::ValidationFailed
                };
                ApiError::new(
                    code,
                    format!("Gagal membaca data file: {}", e),
                    status,
                )
            })?;
            file_data = Some(data.to_vec());
            break;
        }
    }

    let bytes = match file_data {
        Some(b) if !b.is_empty() => b,
        _ => {
            return Err(ApiError::new(
                ErrorCode::ValidationFailed,
                "Field 'image' tidak ditemukan dalam form data atau file kosong.",
                StatusCode::BAD_REQUEST,
            ));
        }
    };

    if bytes.len() > MAX_FILE_SIZE {
        return Err(ApiError::new(
            ErrorCode::PayloadTooLarge,
            "Ukuran file melebihi batas maksimum 5 MB.",
            StatusCode::PAYLOAD_TOO_LARGE,
        ));
    }

    let ext = detect_image_extension(&bytes)?;
    let filename = format!("{}.{}", Uuid::new_v4(), ext);
    let upload_dir = std::path::Path::new("data/uploads");

    if !upload_dir.exists() {
        tokio::fs::create_dir_all(upload_dir).await.map_err(|e| {
            tracing::error!("Gagal membuat direktori uploads: {}", e);
            ApiError::new(
                ErrorCode::InternalError,
                "Gagal menginisialisasi direktori penyimpanan file",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
    }

    let file_path = upload_dir.join(&filename);
    tokio::fs::write(&file_path, &bytes).await.map_err(|e| {
        tracing::error!("Gagal menyimpan file {}: {}", filename, e);
        ApiError::new(
            ErrorCode::InternalError,
            "Gagal menyimpan file gambar ke disk",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;

    tracing::info!(
        filename = %filename,
        size_bytes = %bytes.len(),
        "Product image successfully uploaded and stored"
    );

    Ok(Json(UploadResponse {
        url: format!("/uploads/{}", filename),
        filename,
        size_bytes: bytes.len(),
    }))
}
