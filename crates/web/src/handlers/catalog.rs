use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;
use validator::Validate;

use crate::error::ApiError;
use crate::state::{AppState, ValidatedJson};
use program1_contracts::{
    AuditLogEntry, CatalogItemDto, CreateCatalogItemRequest, ErrorCode, PaginatedResponse,
    PaginationParams,
};

/// List product catalog items with pagination, search, category filter, and sorting
#[utoipa::path(
    get,
    path = "/api/v1/catalog",
    params(
        ("page" = Option<i64>, Query, description = "Page number (min: 1)"),
        ("page_size" = Option<i64>, Query, description = "Page size (1-100, default: 20)"),
        ("search" = Option<String>, Query, description = "Keyword search for product name or SKU"),
        ("category" = Option<String>, Query, description = "Filter by exact category name"),
        ("sort_by" = Option<String>, Query, description = "Field to sort by (name, price, stock, sku, created_at)"),
        ("sort_order" = Option<String>, Query, description = "Sort direction (asc, desc)")
    ),
    responses(
        (status = 200, description = "Paginated catalog items", body = PaginatedResponse<CatalogItemDto>),
        (status = 400, description = "Validation error", body = ApiError)
    ),
    tag = "Catalog"
)]
pub async fn list_catalog(
    Query(params): Query<PaginationParams>,
    State(state): State<AppState>,
) -> Result<Json<PaginatedResponse<CatalogItemDto>>, ApiError> {
    params.validate().map_err(|e| {
        ApiError::new(
            ErrorCode::ValidationFailed,
            format!("Invalid pagination parameters: {}", e),
            StatusCode::BAD_REQUEST,
        )
    })?;

    let items = state
        .catalog_contract
        .list_items_paginated(
            params.page(),
            params.page_size(),
            params.search.as_deref(),
            params.category.as_deref(),
            params.sort_by.as_deref(),
            params.sort_order.as_deref(),
        )
        .await?;
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
    ValidatedJson(mut payload): ValidatedJson<CreateCatalogItemRequest>,
) -> Result<(StatusCode, Json<CatalogItemDto>), ApiError> {
    payload.name = program1_core::sanitize::sanitize_text(&payload.name, 200);
    if let Some(desc) = payload.description {
        payload.description = Some(program1_core::sanitize::sanitize_text(&desc, 2000));
    }

    let sku = payload.sku.clone();
    let item = state.catalog_contract.create_item(payload).await?;

    let _ = state
        .audit_contract
        .log_action(AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor_id: None,
            actor_username: "admin".to_string(),
            action: "CATALOG_ITEM_CREATED".to_string(),
            resource_type: "catalog".to_string(),
            resource_id: Some(item.id),
            details: json!({ "sku": sku, "name": item.name }).to_string(),
            ip_address: None,
        })
        .await;

    Ok((StatusCode::CREATED, Json(item)))
}
