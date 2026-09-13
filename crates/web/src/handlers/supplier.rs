use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use program1_contracts::{
    CreatePurchaseOrderRequest, CreateSupplierRequest, PurchaseOrderDto, SupplierDto,
    UpdateSupplierRequest,
};
use serde::Deserialize;

use crate::error::ApiError;
use crate::middleware::AuthUser;
use crate::state::{AppState, ValidatedJson};

#[derive(Debug, Deserialize)]
pub struct PoListQuery {
    pub status: Option<String>,
}

/// List all suppliers
pub async fn list_suppliers(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<SupplierDto>>, ApiError> {
    let suppliers = state.supplier_contract.list_suppliers().await?;
    Ok(Json(suppliers))
}

/// Get supplier details
pub async fn get_supplier(
    _auth: AuthUser,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<SupplierDto>, ApiError> {
    let supplier = state.supplier_contract.get_supplier(&id).await?;
    Ok(Json(supplier))
}

/// Create a new supplier
pub async fn create_supplier(
    _auth: AuthUser,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateSupplierRequest>,
) -> Result<(StatusCode, Json<SupplierDto>), ApiError> {
    let created = state.supplier_contract.create_supplier(payload).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

/// Update supplier information
pub async fn update_supplier(
    _auth: AuthUser,
    Path(id): Path<String>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<UpdateSupplierRequest>,
) -> Result<Json<SupplierDto>, ApiError> {
    let updated = state.supplier_contract.update_supplier(&id, payload).await?;
    Ok(Json(updated))
}

/// Delete a supplier
pub async fn delete_supplier(
    _auth: AuthUser,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<StatusCode, ApiError> {
    state.supplier_contract.delete_supplier(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// List purchase orders with optional status filter
pub async fn list_purchase_orders(
    _auth: AuthUser,
    Query(query): Query<PoListQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<PurchaseOrderDto>>, ApiError> {
    let orders = state
        .supplier_contract
        .list_purchase_orders(query.status.as_deref())
        .await?;
    Ok(Json(orders))
}

/// Get purchase order detail
pub async fn get_purchase_order(
    _auth: AuthUser,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<PurchaseOrderDto>, ApiError> {
    let order = state.supplier_contract.get_purchase_order(&id).await?;
    Ok(Json(order))
}

/// Create a new purchase order
pub async fn create_purchase_order(
    _auth: AuthUser,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreatePurchaseOrderRequest>,
) -> Result<(StatusCode, Json<PurchaseOrderDto>), ApiError> {
    let order = state.supplier_contract.create_purchase_order(payload).await?;
    Ok((StatusCode::CREATED, Json(order)))
}

/// Mark purchase order as received (triggers stock restock and logs adjustment)
pub async fn receive_purchase_order(
    _auth: AuthUser,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<PurchaseOrderDto>, ApiError> {
    let order = state.supplier_contract.receive_purchase_order(&id).await?;
    Ok(Json(order))
}

/// Cancel a purchase order
pub async fn cancel_purchase_order(
    _auth: AuthUser,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<PurchaseOrderDto>, ApiError> {
    let order = state.supplier_contract.cancel_purchase_order(&id).await?;
    Ok(Json(order))
}
