use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::{AppState, ValidatedJson};
use program1_contracts::{
    AuditLogEntry, CatalogItemDto, CreateCatalogItemRequest,
};

/// List all product catalog items
#[utoipa::path(
    get,
    path = "/api/v1/catalog",
    responses(
        (status = 200, description = "List of catalog items", body = Vec<CatalogItemDto>)
    ),
    tag = "Catalog"
)]
pub async fn list_catalog(
    State(state): State<AppState>,
) -> Result<Json<Vec<CatalogItemDto>>, ApiError> {
    let items = state.catalog_contract.list_items().await?;
    Ok(Json(items))
}

/// Retrieve details of a specific catalog product by ID
#[utoipa::path(
    get,
    path = "/api/v1/catalog/{id}",
    params(
        ("id" = Uuid, Path, description = "Product identifier")
    ),
    responses(
        (status = 200, description = "Catalog item details", body = CatalogItemDto),
        (status = 404, description = "Catalog item not found", body = ApiError)
    ),
    tag = "Catalog"
)]
pub async fn get_catalog_item(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<CatalogItemDto>, ApiError> {
    let item = state.catalog_contract.get_item(id).await?;
    Ok(Json(item))
}

/// Create a new catalog item (Protected)
#[utoipa::path(
    post,
    path = "/api/v1/catalog",
    request_body = CreateCatalogItemRequest,
    responses(
        (status = 201, description = "Catalog item created", body = CatalogItemDto),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 401, description = "Unauthorized"),
        (status = 429, description = "Rate limit exceeded")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Catalog"
)]
pub async fn create_catalog_item(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateCatalogItemRequest>,
) -> Result<(StatusCode, Json<CatalogItemDto>), ApiError> {
    let sku = payload.sku.clone();
    let item = state.catalog_contract.create_item(payload).await?;

    let _ = state.audit_contract.log_action(AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        actor_id: None,
        actor_username: "admin".to_string(),
        action: "CATALOG_ITEM_CREATED".to_string(),
        resource_type: "catalog".to_string(),
        resource_id: Some(item.id),
        details: json!({ "sku": sku, "name": item.name }).to_string(),
        ip_address: None,
    }).await;

    Ok((StatusCode::CREATED, Json(item)))
}
