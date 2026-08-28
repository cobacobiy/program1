use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::{AppState, ValidatedJson};
use program1_contracts::{
    AuditLogEntry, InventoryStockDto, SafetyStockLogDto, UpdateSafetyStockRequest,
};

/// List multi-warehouse inventory stocks & safety levels across all products (Protected)
#[utoipa::path(
    get,
    path = "/api/v1/inventory",
    responses(
        (status = 200, description = "List of inventory stocks", body = Vec<InventoryStockDto>),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Inventory"
)]
pub async fn list_all_inventory(
    State(state): State<AppState>,
) -> Result<Json<Vec<InventoryStockDto>>, ApiError> {
    let stocks = state.inventory_contract.get_all_stocks().await?;
    Ok(Json(stocks))
}

/// Retrieve stock breakdown of a specific product (Protected)
#[utoipa::path(
    get,
    path = "/api/v1/inventory/{id}",
    params(
        ("id" = Uuid, Path, description = "Product identifier")
    ),
    responses(
        (status = 200, description = "Inventory stock details", body = InventoryStockDto),
        (status = 404, description = "Product stock not found", body = ApiError),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Inventory"
)]
pub async fn get_inventory_stock(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<InventoryStockDto>, ApiError> {
    let stock = state.inventory_contract.get_stock(id).await?;
    Ok(Json(stock))
}

/// Update safety stock threshold for a product and record audit note (Protected)
#[utoipa::path(
    post,
    path = "/api/v1/inventory/{id}/safety-stock",
    params(
        ("id" = Uuid, Path, description = "Product identifier")
    ),
    request_body = UpdateSafetyStockRequest,
    responses(
        (status = 200, description = "Safety stock updated successfully", body = InventoryStockDto),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 404, description = "Product stock not found", body = ApiError),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Inventory"
)]
pub async fn update_safety_stock(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<UpdateSafetyStockRequest>,
) -> Result<Json<InventoryStockDto>, ApiError> {
    let operator = payload.updated_by.unwrap_or_else(|| "Admin Ginee".to_string());
    let updated = state
        .inventory_contract
        .update_safety_stock(id, payload.new_safety_stock, payload.admin_note.clone(), operator.clone())
        .await?;

    let _ = state.audit_contract.log_action(AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        actor_id: None,
        actor_username: operator,
        action: "SAFETY_STOCK_UPDATED".to_string(),
        resource_type: "inventory".to_string(),
        resource_id: Some(id),
        details: json!({ "new_safety_stock": payload.new_safety_stock, "note": payload.admin_note }).to_string(),
        ip_address: None,
    }).await;

    Ok(Json(updated))
}

/// Retrieve history logs of safety stock adjustments for a product (Protected)
#[utoipa::path(
    get,
    path = "/api/v1/inventory/{id}/safety-stock-logs",
    params(
        ("id" = Uuid, Path, description = "Product identifier")
    ),
    responses(
        (status = 200, description = "List of safety stock adjustment logs", body = Vec<SafetyStockLogDto>),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Inventory"
)]
pub async fn get_safety_stock_logs(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<Vec<SafetyStockLogDto>>, ApiError> {
    let logs = state.inventory_contract.get_safety_stock_logs(id).await?;
    Ok(Json(logs))
}
