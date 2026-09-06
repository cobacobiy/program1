use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::{AppState, ValidatedJson};
use program1_contracts::{
    AuditLogEntry, BuyerCheckoutRequest, ChannelType, ErrorCode, JwtClaims, MarketplaceOrderReq,
    OmniOrderDto, ShippingAddressSnapshot, StorefrontOrderRequest,
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

/// Place a new order via buyer storefront checkout (Requires Buyer Auth)
#[utoipa::path(
    post,
    path = "/api/v1/orders",
    request_body = BuyerCheckoutRequest,
    responses(
        (status = 201, description = "Order placed successfully", body = OmniOrderDto),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Shipping address not found"),
        (status = 409, description = "Insufficient inventory stock", body = ApiError),
        (status = 422, description = "Phone not verified", body = ApiError),
        (status = 429, description = "Rate limit exceeded")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Orders"
)]
pub async fn create_storefront_order(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    ValidatedJson(payload): ValidatedJson<BuyerCheckoutRequest>,
) -> Result<(StatusCode, Json<OmniOrderDto>), ApiError> {
    if !claims.is_buyer() {
        return Err(ApiError::new(
            ErrorCode::InsufficientPermissions,
            "Checkout storefront hanya dapat dilakukan oleh akun pembeli",
            StatusCode::FORBIDDEN,
        ));
    }

    let buyer_id = claims.sub;

    // 1. Confirm buyer exists and phone is verified
    let buyer = state.buyer_contract.get_buyer_profile(buyer_id).await?;
    if !buyer.phone_verified {
        return Err(ApiError::new(
            ErrorCode::InvalidRequest,
            "Nomor HP wajib diverifikasi terlebih dahulu sebelum checkout",
            StatusCode::UNPROCESSABLE_ENTITY,
        ));
    }

    // 2. Load address server-side & confirm buyer ownership
    let address = state
        .buyer_contract
        .get_address(buyer_id, payload.address_id)
        .await
        .map_err(|e| match e {
            program1_contracts::ContractError::NotFound(_) => ApiError::new(
                ErrorCode::ResourceNotFound,
                "Alamat pengiriman tidak ditemukan atau bukan milik akun Anda",
                StatusCode::NOT_FOUND,
            ),
            other => ApiError::from(other),
        })?;

    // 3. Construct immutable shipping snapshot server-side
    let shipping_snapshot = ShippingAddressSnapshot {
        recipient_name: address.recipient_name.clone(),
        phone_number: address.phone_number.clone(),
        street_address: address.street_address.clone(),
        subdistrict: address.subdistrict.clone(),
        city: address.city.clone(),
        province: address.province.clone(),
        postal_code: address.postal_code.clone(),
    };

    let formatted_address = format!(
        "{}, {}, {}, {} {}",
        address.street_address,
        address.subdistrict,
        address.city,
        address.province,
        address.postal_code
    );

    let storefront_req = StorefrontOrderRequest {
        customer_name: address.recipient_name.clone(),
        customer_email: buyer.email.clone(),
        shipping_address: formatted_address,
        items: payload.items,
        buyer_id: Some(buyer_id),
        shipping_snapshot: Some(shipping_snapshot),
    };

    let order = state
        .order_contract
        .create_storefront_order(storefront_req)
        .await?;

    let _ = state
        .audit_contract
        .log_action(AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor_id: Some(buyer_id),
            actor_username: program1_module_buyer::mask_email(&buyer.email),
            action: "ORDER_CREATED".to_string(),
            resource_type: "order".to_string(),
            resource_id: Some(order.id),
            details: json!({
                "total_amount": order.total_amount,
                "channel": "NativeWeb",
                "recipient_name": address.recipient_name,
                "address_id": payload.address_id,
            })
            .to_string(),
            ip_address: None,
        })
        .await;

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
        .create_marketplace_order(
            channel.clone(),
            payload.customer_name.clone(),
            payload.items,
        )
        .await?;

    let _ = state
        .audit_contract
        .log_action(AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor_id: None,
            actor_username: order.customer_name.clone(),
            action: "ORDER_CREATED".to_string(),
            resource_type: "order".to_string(),
            resource_id: Some(order.id),
            details: json!({ "total_amount": order.total_amount, "channel": channel.to_string() })
                .to_string(),
            ip_address: None,
        })
        .await;

    Ok((StatusCode::CREATED, Json(order)))
}
