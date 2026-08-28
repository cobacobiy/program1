# Issue #8 — Error Handling Standardization

> **Prioritas**: 🟢 MEDIUM
> **Estimasi**: 1-2 hari
> **Depends On**: —
> **Blocks**: —

---

## 🔍 Masalah Saat Ini

Error handling saat ini:
- `ContractError` sudah bagus tapi mapping ke HTTP response tidak konsisten
- Beberapa error masih pakai generic `Internal(String)` tanpa detail
- Tidak ada error ID/code untuk referensi frontend
- Stack trace bisa bocor ke client di production
- Panic bisa crash seluruh server (tidak ada panic handler)

### Contoh Inkonsistensi:
```rust
// Kadang return bare tuple
(StatusCode::NOT_FOUND, Json(json!({ "error": msg })))

// Kadang return ContractError mapping
map_contract_error(e)
```

---

## ✅ Acceptance Criteria

### 1. Standardized Error Response Format

Semua error response harus mengikuti format:

```json
{
    "error": {
        "code": "RESOURCE_NOT_FOUND",
        "message": "Catalog Item 10000000-0000-0000-0000-000000000099 not found",
        "status": 404,
        "request_id": "req_a1b2c3d4"
    }
}
```

### 2. Definisikan Error Codes

```rust
// Di crates/contracts/src/lib.rs atau file baru crates/contracts/src/errors.rs

pub enum ErrorCode {
    // 400
    ValidationFailed,
    InvalidRequest,
    
    // 401
    AuthenticationRequired,
    InvalidCredentials,
    TokenExpired,
    
    // 403
    InsufficientPermissions,
    
    // 404
    ResourceNotFound,
    
    // 409
    InsufficientStock,
    DuplicateResource,
    
    // 429
    RateLimitExceeded,
    
    // 500
    InternalError,
    
    // 502
    ChannelSyncFailed,
}
```

### 3. Buat `ApiError` Wrapper

Di `crates/web/src/main.rs` atau file terpisah:

```rust
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
    pub status: StatusCode,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = json!({
            "error": {
                "code": self.code.as_str(),
                "message": self.message,
                "status": self.status.as_u16(),
            }
        });
        (self.status, Json(body)).into_response()
    }
}

// Auto-convert dari ContractError
impl From<ContractError> for ApiError {
    fn from(err: ContractError) -> Self {
        match err {
            ContractError::NotFound(msg) => ApiError {
                code: ErrorCode::ResourceNotFound,
                message: msg,
                status: StatusCode::NOT_FOUND,
            },
            ContractError::ValidationError(msg) => ApiError {
                code: ErrorCode::ValidationFailed,
                message: msg,
                status: StatusCode::BAD_REQUEST,
            },
            // ... etc
        }
    }
}
```

### 4. Simplify Handler Return Types

```rust
// SEBELUM — verbose match di setiap handler:
async fn list_catalog(State(state): State<AppState>) -> impl IntoResponse {
    match state.catalog_contract.list_items().await {
        Ok(items) => (StatusCode::OK, Json(json!(items))),
        Err(e) => map_contract_error(e),
    }
}

// SESUDAH — clean Result-based:
async fn list_catalog(
    State(state): State<AppState>,
) -> Result<Json<Vec<CatalogItemDto>>, ApiError> {
    let items = state.catalog_contract.list_items().await?;
    Ok(Json(items))
}
```

### 5. Panic Handler / Catch-Unwind Layer

```rust
use tower_http::catch_panic::CatchPanicLayer;

let app = Router::new()
    // ... routes
    .layer(CatchPanicLayer::custom(|_| {
        let body = json!({
            "error": {
                "code": "INTERNAL_ERROR",
                "message": "An unexpected error occurred",
                "status": 500
            }
        });
        (StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response()
    }));
```

### 6. Hide Internal Errors di Production

```rust
impl From<ContractError> for ApiError {
    fn from(err: ContractError) -> Self {
        match err {
            ContractError::Internal(msg) => {
                // Log actual error
                tracing::error!(error = %msg, "Internal error");
                
                // Don't expose internals to client
                ApiError {
                    code: ErrorCode::InternalError,
                    message: if cfg!(debug_assertions) {
                        msg // Show in dev
                    } else {
                        "An internal error occurred".to_string() // Hide in prod
                    },
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                }
            },
            // ...
        }
    }
}
```

---

## 🧪 Unit Test

```rust
#[cfg(test)]
mod tests {
    // 1. Test ContractError to ApiError conversion
    #[test]
    fn test_not_found_maps_to_404() { ... }
    
    // 2. Test error response format
    #[test]
    fn test_error_response_has_correct_structure() { ... }
    
    // 3. Test internal errors hide details
    #[test]
    fn test_internal_error_hides_details_in_release() { ... }
}
```

---

## ⚠️ Peringatan

- Semua existing handlers harus di-update — ini adalah refactor yang menyentuh semua handler di `main.rs`
- Jangan ubah `ContractError` enum values — hanya tambahkan mapping
- Semua existing test `cargo test --workspace` harus tetap PASS
