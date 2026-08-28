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

/// List all omnichannel orders in history (Protected)
#[utoipa::path(
    get,
    path = "/api/v1/orders",
    responses(
        (status = 200, description = "List of orders", body = Vec<OmniOrderDto>),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Orders"
)]
pub async fn list_orders(
    State(state): State<AppState>,
) -> Result<Json<Vec<OmniOrderDto>>, ApiError> {
    let orders = state.order_contract.list_orders().await?;
    Ok(Json(orders))
}

/// Retrieve details of a specific order by ID (Protected)
#[utoipa::path(
    get,
    path = "/api/v1/orders/{id}",
    params(
        ("id" = Uuid, Path, description = "Order identifier")
    ),
    responses(
        (status = 200, description = "Order details", body = OmniOrderDto),
        (status = 404, description = "Order not found", body = ApiError),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Orders"
)]
pub async fn get_order(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<OmniOrderDto>, ApiError> {
    let order = state.order_contract.get_order(id).await?;
    Ok(Json(order))
}

/// Place a new order via public storefront checkout
#[utoipa::path(
    post,
    path = "/api/v1/orders",
    request_body = StorefrontOrderRequest,
    responses(
        (status = 201, description = "Order placed successfully", body = OmniOrderDto),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 409, description = "Insufficient inventory stock", body = ApiError),
        (status = 429, description = "Rate limit exceeded")
    ),
    tag = "Orders"
)]
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

/// Ingest remote order from integrated marketplace (Protected)
#[utoipa::path(
    post,
    path = "/api/v1/orders/marketplace",
    request_body = MarketplaceOrderReq,
    responses(
        (status = 201, description = "Marketplace order recorded", body = OmniOrderDto),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Insufficient stock", body = ApiError)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Orders"
)]
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
