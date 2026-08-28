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

pub async fn list_all_inventory(
    State(state): State<AppState>,
) -> Result<Json<Vec<InventoryStockDto>>, ApiError> {
    let stocks = state.inventory_contract.get_all_stocks().await?;
    Ok(Json(stocks))
}

pub async fn get_inventory_stock(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<InventoryStockDto>, ApiError> {
    let stock = state.inventory_contract.get_stock(id).await?;
    Ok(Json(stock))
}

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

pub async fn get_safety_stock_logs(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<Vec<SafetyStockLogDto>>, ApiError> {
    let logs = state.inventory_contract.get_safety_stock_logs(id).await?;
    Ok(Json(logs))
}
