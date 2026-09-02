use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::{AppState, ValidatedJson};
use program1_contracts::{
    AuditLogEntry, BulkStockUpdateRequest, BulkStockUpdateResult, InventoryStockDto, JwtClaims,
    LowStockAlertDto, SafetyStockLogDto, StockAdjustmentLogDto, UpdatePromotionStockRequest,
    UpdateSafetyStockRequest, UpdateSpareStockRequest, UpdateWarehouseStockRequest,
};

#[derive(Debug, Deserialize)]
pub struct AdjustmentLogQuery {
    pub adjustment_type: Option<String>,
}

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

/// Update safety stock threshold for a product and record audit note (Admin only)
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
        (status = 403, description = "Forbidden (Admin only)", body = ApiError),
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
    Extension(claims): Extension<JwtClaims>,
    ValidatedJson(payload): ValidatedJson<UpdateSafetyStockRequest>,
) -> Result<Json<InventoryStockDto>, ApiError> {
    let operator = claims.username.clone();
    let updated = state
        .inventory_contract
        .update_safety_stock(id, payload.new_safety_stock, payload.admin_note.clone(), operator.clone())
        .await?;

    if let Err(e) = state.audit_contract.log_action(AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        actor_id: Some(claims.sub),
        actor_username: operator,
        action: "SAFETY_STOCK_UPDATED".to_string(),
        resource_type: "inventory".to_string(),
        resource_id: Some(id),
        details: json!({ "new_safety_stock": payload.new_safety_stock, "note": payload.admin_note }).to_string(),
        ip_address: None,
    }).await {
        tracing::warn!(error = %e, "Failed to write audit log for safety stock update");
    }

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

/// Update warehouse stock for a product with audit note (Admin only)
#[utoipa::path(
    post,
    path = "/api/v1/inventory/{id}/warehouse-stock",
    params(
        ("id" = Uuid, Path, description = "Product identifier")
    ),
    request_body = UpdateWarehouseStockRequest,
    responses(
        (status = 200, description = "Warehouse stock updated successfully", body = InventoryStockDto),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 403, description = "Forbidden (Admin only)", body = ApiError),
        (status = 404, description = "Product stock not found", body = ApiError),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Inventory"
)]
pub async fn update_warehouse_stock(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    ValidatedJson(payload): ValidatedJson<UpdateWarehouseStockRequest>,
) -> Result<Json<InventoryStockDto>, ApiError> {
    let operator = claims.username.clone();
    let updated = state
        .inventory_contract
        .update_warehouse_stock(id, payload.new_warehouse_stock, payload.admin_note.clone(), operator.clone())
        .await?;

    if let Err(e) = state.audit_contract.log_action(AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        actor_id: Some(claims.sub),
        actor_username: operator,
        action: "WAREHOUSE_STOCK_UPDATED".to_string(),
        resource_type: "inventory".to_string(),
        resource_id: Some(id),
        details: json!({ "new_warehouse_stock": payload.new_warehouse_stock, "note": payload.admin_note }).to_string(),
        ip_address: None,
    }).await {
        tracing::warn!(error = %e, "Failed to write audit log for warehouse stock update");
    }

    Ok(Json(updated))
}

/// Update spare stock for a product (Admin only)
#[utoipa::path(
    post,
    path = "/api/v1/inventory/{id}/spare-stock",
    params(
        ("id" = Uuid, Path, description = "Product identifier")
    ),
    request_body = UpdateSpareStockRequest,
    responses(
        (status = 200, description = "Spare stock updated successfully", body = InventoryStockDto),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 403, description = "Forbidden (Admin only)", body = ApiError),
        (status = 404, description = "Product stock not found", body = ApiError),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Inventory"
)]
pub async fn update_spare_stock(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    ValidatedJson(payload): ValidatedJson<UpdateSpareStockRequest>,
) -> Result<Json<InventoryStockDto>, ApiError> {
    let operator = claims.username.clone();
    let updated = state
        .inventory_contract
        .update_spare_stock(id, payload.new_spare_stock, payload.admin_note.clone(), operator.clone())
        .await?;

    if let Err(e) = state.audit_contract.log_action(AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        actor_id: Some(claims.sub),
        actor_username: operator,
        action: "SPARE_STOCK_UPDATED".to_string(),
        resource_type: "inventory".to_string(),
        resource_id: Some(id),
        details: json!({ "new_spare_stock": payload.new_spare_stock, "note": payload.admin_note }).to_string(),
        ip_address: None,
    }).await {
        tracing::warn!(error = %e, "Failed to write audit log for spare stock update");
    }

    Ok(Json(updated))
}

/// Update promotion stock for a product (Admin only)
#[utoipa::path(
    post,
    path = "/api/v1/inventory/{id}/promotion-stock",
    params(
        ("id" = Uuid, Path, description = "Product identifier")
    ),
    request_body = UpdatePromotionStockRequest,
    responses(
        (status = 200, description = "Promotion stock updated successfully", body = InventoryStockDto),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 403, description = "Forbidden (Admin only)", body = ApiError),
        (status = 404, description = "Product stock not found", body = ApiError),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Inventory"
)]
pub async fn update_promotion_stock(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    ValidatedJson(payload): ValidatedJson<UpdatePromotionStockRequest>,
) -> Result<Json<InventoryStockDto>, ApiError> {
    let operator = claims.username.clone();
    let updated = state
        .inventory_contract
        .update_promotion_stock(id, payload.new_promotion_stock, payload.admin_note.clone(), operator.clone())
        .await?;

    if let Err(e) = state.audit_contract.log_action(AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        actor_id: Some(claims.sub),
        actor_username: operator,
        action: "PROMOTION_STOCK_UPDATED".to_string(),
        resource_type: "inventory".to_string(),
        resource_id: Some(id),
        details: json!({ "new_promotion_stock": payload.new_promotion_stock, "note": payload.admin_note }).to_string(),
        ip_address: None,
    }).await {
        tracing::warn!(error = %e, "Failed to write audit log for promotion stock update");
    }

    Ok(Json(updated))
}

/// Retrieve unified adjustment history logs for a product (Protected)
#[utoipa::path(
    get,
    path = "/api/v1/inventory/{id}/adjustment-logs",
    params(
        ("id" = Uuid, Path, description = "Product identifier"),
        ("adjustment_type" = Option<String>, Query, description = "Filter by type (warehouse/safety/spare/promotion)")
    ),
    responses(
        (status = 200, description = "List of stock adjustment logs", body = Vec<StockAdjustmentLogDto>),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Inventory"
)]
pub async fn get_adjustment_logs(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Query(query): Query<AdjustmentLogQuery>,
) -> Result<Json<Vec<StockAdjustmentLogDto>>, ApiError> {
    let logs = state
        .inventory_contract
        .get_adjustment_logs(id, query.adjustment_type.as_deref())
        .await?;
    Ok(Json(logs))
}

/// List low stock alerts across all products where available <= safety stock (Protected)
#[utoipa::path(
    get,
    path = "/api/v1/inventory/alerts/low-stock",
    responses(
        (status = 200, description = "List of low stock alert items", body = Vec<LowStockAlertDto>),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Inventory"
)]
pub async fn get_low_stock_alerts(
    State(state): State<AppState>,
) -> Result<Json<Vec<LowStockAlertDto>>, ApiError> {
    let alerts = state.inventory_contract.get_low_stock_alerts().await?;
    Ok(Json(alerts))
}

/// Bulk update inventory stock for multiple products (Admin only)
#[utoipa::path(
    post,
    path = "/api/v1/inventory/bulk-update",
    request_body = BulkStockUpdateRequest,
    responses(
        (status = 200, description = "Bulk update execution summary", body = BulkStockUpdateResult),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 403, description = "Forbidden (Admin only)", body = ApiError),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Inventory"
)]
pub async fn bulk_update_stock(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    ValidatedJson(payload): ValidatedJson<BulkStockUpdateRequest>,
) -> Result<Json<BulkStockUpdateResult>, ApiError> {
    let operator = claims.username.clone();
    let note = payload.admin_note.clone();
    let count = payload.adjustments.len();

    let mut update_req = payload;
    update_req.updated_by = Some(operator.clone());

    let result = state.inventory_contract.bulk_update_stock(update_req).await?;

    if let Err(e) = state.audit_contract.log_action(AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        actor_id: Some(claims.sub),
        actor_username: operator,
        action: "BULK_STOCK_UPDATED".to_string(),
        resource_type: "inventory".to_string(),
        resource_id: None,
        details: json!({
            "total_requested": count,
            "success": result.total_success,
            "failed": result.total_failed,
            "note": note
        }).to_string(),
        ip_address: None,
    }).await {
        tracing::warn!(error = %e, "Failed to write audit log for bulk stock update");
    }

    Ok(Json(result))
}
