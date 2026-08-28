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
    AuditLogEntry, ChannelType, MarketplaceOrderReq, OmniOrderDto, StorefrontOrderRequest,
};

pub async fn list_orders(
    State(state): State<AppState>,
) -> Result<Json<Vec<OmniOrderDto>>, ApiError> {
    let orders = state.order_contract.list_orders().await?;
    Ok(Json(orders))
}

pub async fn get_order(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<OmniOrderDto>, ApiError> {
    let order = state.order_contract.get_order(id).await?;
    Ok(Json(order))
}

pub async fn create_storefront_order(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<StorefrontOrderRequest>,
) -> Result<(StatusCode, Json<OmniOrderDto>), ApiError> {
    let order = state.order_contract.create_storefront_order(payload).await?;

    let _ = state.audit_contract.log_action(AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        actor_id: None,
        actor_username: order.customer_name.clone(),
        action: "ORDER_CREATED".to_string(),
        resource_type: "order".to_string(),
        resource_id: Some(order.id),
        details: json!({ "total_amount": order.total_amount, "channel": "NativeWeb" }).to_string(),
        ip_address: None,
    }).await;

    Ok((StatusCode::CREATED, Json(order)))
}

pub async fn create_marketplace_order(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<MarketplaceOrderReq>,
) -> Result<(StatusCode, Json<OmniOrderDto>), ApiError> {
    let channel = match payload.channel.to_lowercase().as_str() {
        "tiktok" | "tiktokshop" => ChannelType::TikTokShop,
        "shopee" => ChannelType::Shopee,
        "tokopedia" => ChannelType::Tokopedia,
        _ => ChannelType::NativeWeb,
    };

    let order = state
        .order_contract
        .create_marketplace_order(channel.clone(), payload.customer_name.clone(), payload.items)
        .await?;

    let _ = state.audit_contract.log_action(AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        actor_id: None,
        actor_username: order.customer_name.clone(),
        action: "ORDER_CREATED".to_string(),
        resource_type: "order".to_string(),
        resource_id: Some(order.id),
        details: json!({ "total_amount": order.total_amount, "channel": channel.to_string() }).to_string(),
        ip_address: None,
    }).await;

    Ok((StatusCode::CREATED, Json(order)))
}
