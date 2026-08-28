# Issue #11 — API Versioning & Documentation (OpenAPI)

> **Prioritas**: 🟢 MEDIUM
> **Estimasi**: 2 hari
> **Depends On**: —
> **Blocks**: —

---

## 🔍 Masalah Saat Ini

- API sudah pakai prefix `/api/v1/` tapi tidak ada mekanisme versioning formal
- Tidak ada API documentation — developer harus baca source code
- Tidak ada Swagger/OpenAPI spec untuk testing
- Frontend developer tidak punya referensi endpoint yang bisa dipercaya

---

## ✅ Acceptance Criteria

### 1. Tambahkan Dependencies

```toml
# Cargo.toml [workspace.dependencies]
utoipa = { version = "4", features = ["axum_extras", "uuid", "chrono"] }
utoipa-swagger-ui = { version = "7", features = ["axum"] }
```

### 2. Annotate DTOs dengan `utoipa::ToSchema`

```rust
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CatalogItemDto {
    /// Unique product identifier
    pub id: Uuid,
    /// Product display name
    pub name: String,
    // ... etc
}
```

### 3. Annotate Handlers dengan `#[utoipa::path]`

```rust
/// List all catalog items
#[utoipa::path(
    get,
    path = "/api/v1/catalog",
    responses(
        (status = 200, description = "List of catalog items", body = Vec<CatalogItemDto>),
    ),
    tag = "Catalog"
)]
async fn list_catalog(...) { ... }
```

### 4. Generate OpenAPI Spec

```rust
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        health_check,
        list_catalog,
        get_catalog_item,
        create_catalog_item,
        // ... all endpoints
    ),
    components(schemas(
        CatalogItemDto,
        CreateCatalogItemRequest,
        // ... all DTOs
    )),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Auth", description = "Authentication endpoints"),
        (name = "Catalog", description = "Product catalog management"),
        (name = "Inventory", description = "Ginee OMS inventory management"),
        (name = "Orders", description = "Order management"),
        (name = "Channels", description = "Marketplace channel sync"),
        (name = "Analytics", description = "Sales analytics"),
        (name = "Users", description = "User account & RBAC"),
    ),
    info(
        title = "Program1 — Omnichannel Commerce API",
        version = "1.0.0",
        description = "Modular Monolith REST API for AURA Storefront"
    )
)]
struct ApiDoc;
```

### 5. Serve Swagger UI

```rust
use utoipa_swagger_ui::SwaggerUi;

let app = Router::new()
    // ... existing routes
    .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()));
```

Swagger UI akan bisa diakses di: `http://localhost:8080/swagger-ui/`

### 6. API Version Header

Tambahkan middleware yang set header:
```
X-API-Version: 1.0.0
```

---

## 🧪 Unit Test

```rust
#[cfg(test)]
mod tests {
    // 1. Test OpenAPI spec valid
    #[test]
    fn test_openapi_spec_valid() {
        let spec = ApiDoc::openapi();
        let json = serde_json::to_string_pretty(&spec).unwrap();
        assert!(json.contains("Program1"));
    }
    
    // 2. Test all endpoints documented
    #[test]
    fn test_all_endpoints_in_spec() { ... }
}
```

---

## ⚠️ Peringatan

- Swagger UI hanya di-enable di `development` dan `staging` — DISABLE di production
- OpenAPI spec bisa di-export sebagai JSON/YAML untuk frontend codegen
- Semua existing test `cargo test --workspace` harus tetap PASS

---

## 📎 Referensi

- [utoipa docs](https://docs.rs/utoipa/latest/utoipa/)
- [utoipa-swagger-ui](https://docs.rs/utoipa-swagger-ui/latest/utoipa_swagger_ui/)
