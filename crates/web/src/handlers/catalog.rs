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
    AuditLogEntry, CatalogItemDto, CreateCatalogItemRequest, CreateVariantRequest, ErrorCode,
    PaginatedResponse, PaginationParams, ProductVariantDto, UpdateVariantRequest,
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

/// List all active variants for a specific catalog product
#[utoipa::path(
    get,
    path = "/api/v1/catalog/{id}/variants",
    params(
        ("id" = Uuid, Path, description = "Product identifier")
    ),
    responses(
        (status = 200, description = "List of product variants", body = Vec<ProductVariantDto>),
        (status = 404, description = "Catalog item not found", body = ApiError)
    ),
    tag = "Catalog"
)]
pub async fn list_variants_handler(
    Path(product_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<Vec<ProductVariantDto>>, ApiError> {
    let variants = state.catalog_contract.list_variants(product_id).await?;
    Ok(Json(variants))
}

/// Get details of a specific product variant
#[utoipa::path(
    get,
    path = "/api/v1/catalog/{id}/variants/{vid}",
    params(
        ("id" = Uuid, Path, description = "Product identifier"),
        ("vid" = Uuid, Path, description = "Variant identifier")
    ),
    responses(
        (status = 200, description = "Product variant details", body = ProductVariantDto),
        (status = 404, description = "Product variant not found", body = ApiError)
    ),
    tag = "Catalog"
)]
pub async fn get_variant_handler(
    Path((_product_id, variant_id)): Path<(Uuid, Uuid)>,
    State(state): State<AppState>,
) -> Result<Json<ProductVariantDto>, ApiError> {
    let variant = state.catalog_contract.get_variant(variant_id).await?;
    Ok(Json(variant))
}

/// Create a new variant for a catalog product (Protected)
#[utoipa::path(
    post,
    path = "/api/v1/catalog/{id}/variants",
    params(
        ("id" = Uuid, Path, description = "Product identifier")
    ),
    request_body = CreateVariantRequest,
    responses(
        (status = 201, description = "Product variant created", body = ProductVariantDto),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Catalog item not found", body = ApiError)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Catalog"
)]
pub async fn create_variant_handler(
    Path(product_id): Path<Uuid>,
    State(state): State<AppState>,
    ValidatedJson(mut payload): ValidatedJson<CreateVariantRequest>,
) -> Result<(StatusCode, Json<ProductVariantDto>), ApiError> {
    payload.variant_name = program1_core::sanitize::sanitize_text(&payload.variant_name, 50);
    payload.variant_value = program1_core::sanitize::sanitize_text(&payload.variant_value, 100);

    let variant = state.catalog_contract.create_variant(product_id, payload).await?;

    let _ = state
        .audit_contract
        .log_action(AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor_id: None,
            actor_username: "admin".to_string(),
            action: "CATALOG_VARIANT_CREATED".to_string(),
            resource_type: "catalog_variant".to_string(),
            resource_id: Some(variant.id),
            details: json!({
                "product_id": product_id,
                "variant_name": variant.variant_name,
                "variant_value": variant.variant_value,
                "sku": variant.sku
            })
            .to_string(),
            ip_address: None,
        })
        .await;

    Ok((StatusCode::CREATED, Json(variant)))
}

/// Update an existing product variant (Protected)
#[utoipa::path(
    put,
    path = "/api/v1/catalog/{id}/variants/{vid}",
    params(
        ("id" = Uuid, Path, description = "Product identifier"),
        ("vid" = Uuid, Path, description = "Variant identifier")
    ),
    request_body = UpdateVariantRequest,
    responses(
        (status = 200, description = "Product variant updated", body = ProductVariantDto),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Variant not found", body = ApiError)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Catalog"
)]
pub async fn update_variant_handler(
    Path((product_id, variant_id)): Path<(Uuid, Uuid)>,
    State(state): State<AppState>,
    ValidatedJson(mut payload): ValidatedJson<UpdateVariantRequest>,
) -> Result<Json<ProductVariantDto>, ApiError> {
    payload.variant_name = program1_core::sanitize::sanitize_text(&payload.variant_name, 50);
    payload.variant_value = program1_core::sanitize::sanitize_text(&payload.variant_value, 100);

    let variant = state.catalog_contract.update_variant(variant_id, payload).await?;

    let _ = state
        .audit_contract
        .log_action(AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor_id: None,
            actor_username: "admin".to_string(),
            action: "CATALOG_VARIANT_UPDATED".to_string(),
            resource_type: "catalog_variant".to_string(),
            resource_id: Some(variant.id),
            details: json!({
                "product_id": product_id,
                "variant_name": variant.variant_name,
                "variant_value": variant.variant_value
            })
            .to_string(),
            ip_address: None,
        })
        .await;

    Ok(Json(variant))
}

/// Delete a product variant (Protected)
#[utoipa::path(
    delete,
    path = "/api/v1/catalog/{id}/variants/{vid}",
    params(
        ("id" = Uuid, Path, description = "Product identifier"),
        ("vid" = Uuid, Path, description = "Variant identifier")
    ),
    responses(
        (status = 204, description = "Product variant deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Variant not found", body = ApiError)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Catalog"
)]
pub async fn delete_variant_handler(
    Path((product_id, variant_id)): Path<(Uuid, Uuid)>,
    State(state): State<AppState>,
) -> Result<StatusCode, ApiError> {
    state.catalog_contract.delete_variant(variant_id).await?;

    let _ = state
        .audit_contract
        .log_action(AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor_id: None,
            actor_username: "admin".to_string(),
            action: "CATALOG_VARIANT_DELETED".to_string(),
            resource_type: "catalog_variant".to_string(),
            resource_id: Some(variant_id),
            details: json!({ "product_id": product_id }).to_string(),
            ip_address: None,
        })
        .await;

    Ok(StatusCode::NO_CONTENT)
}

