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
    AuditLogEntry, CategoryDto, CreateCategoryRequest, UpdateCategoryRequest,
};

/// List all product categories
#[utoipa::path(
    get,
    path = "/api/v1/categories",
    responses(
        (status = 200, description = "List of product categories", body = Vec<CategoryDto>),
    ),
    tag = "Catalog"
)]
pub async fn list_categories_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<CategoryDto>>, ApiError> {
    let categories = state.catalog_contract.list_categories().await?;
    Ok(Json(categories))
}

/// Get details of a category by ID or slug
#[utoipa::path(
    get,
    path = "/api/v1/categories/{id}",
    params(
        ("id" = String, Path, description = "Category ID or Slug")
    ),
    responses(
        (status = 200, description = "Category details", body = CategoryDto),
        (status = 404, description = "Category not found", body = ApiError)
    ),
    tag = "Catalog"
)]
pub async fn get_category_handler(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<CategoryDto>, ApiError> {
    let cat = state.catalog_contract.get_category(&id).await?;
    Ok(Json(cat))
}

/// Create a new category (Admin only)
#[utoipa::path(
    post,
    path = "/api/v1/admin/categories",
    request_body = CreateCategoryRequest,
    responses(
        (status = 201, description = "Category created", body = CategoryDto),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Catalog"
)]
pub async fn create_category_handler(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateCategoryRequest>,
) -> Result<(StatusCode, Json<CategoryDto>), ApiError> {
    let category = state.catalog_contract.create_category(payload).await?;

    let _ = state
        .audit_contract
        .log_action(AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor_id: None,
            actor_username: "admin".to_string(),
            action: "CATEGORY_CREATED".to_string(),
            resource_type: "category".to_string(),
            resource_id: None,
            details: json!({ "id": category.id, "name": category.name, "slug": category.slug }).to_string(),
            ip_address: None,
        })
        .await;

    Ok((StatusCode::CREATED, Json(category)))
}

/// Update an existing category (Admin only)
#[utoipa::path(
    put,
    path = "/api/v1/admin/categories/{id}",
    params(
        ("id" = String, Path, description = "Category identifier")
    ),
    request_body = UpdateCategoryRequest,
    responses(
        (status = 200, description = "Category updated", body = CategoryDto),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 404, description = "Category not found", body = ApiError)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Catalog"
)]
pub async fn update_category_handler(
    Path(id): Path<String>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<UpdateCategoryRequest>,
) -> Result<Json<CategoryDto>, ApiError> {
    let category = state.catalog_contract.update_category(&id, payload).await?;

    let _ = state
        .audit_contract
        .log_action(AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor_id: None,
            actor_username: "admin".to_string(),
            action: "CATEGORY_UPDATED".to_string(),
            resource_type: "category".to_string(),
            resource_id: None,
            details: json!({ "id": category.id, "name": category.name }).to_string(),
            ip_address: None,
        })
        .await;

    Ok(Json(category))
}

/// Delete a category (Admin only, blocked if products are assigned)
#[utoipa::path(
    delete,
    path = "/api/v1/admin/categories/{id}",
    params(
        ("id" = String, Path, description = "Category identifier")
    ),
    responses(
        (status = 200, description = "Category deleted successfully"),
        (status = 400, description = "Category has assigned products or validation error", body = ApiError),
        (status = 404, description = "Category not found", body = ApiError)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Catalog"
)]
pub async fn delete_category_handler(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.catalog_contract.delete_category(&id).await?;

    let _ = state
        .audit_contract
        .log_action(AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor_id: None,
            actor_username: "admin".to_string(),
            action: "CATEGORY_DELETED".to_string(),
            resource_type: "category".to_string(),
            resource_id: None,
            details: json!({ "id": id }).to_string(),
            ip_address: None,
        })
        .await;

    Ok(Json(json!({
        "status": "success",
        "message": format!("Category {} deleted successfully", id)
    })))
}
