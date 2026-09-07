use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::{AppState, ValidatedJson};
use program1_contracts::{
    ContractError, CreatePaymentRequest, ErrorCode, JwtClaims, PaymentConfigDto,
    PaymentTransactionDto,
};

/// Create payment intent for an order (Buyer only)
#[utoipa::path(
    post,
    path = "/api/v1/payments",
    request_body = CreatePaymentRequest,
    responses(
        (status = 201, description = "Payment transaction created and Snap token generated", body = PaymentTransactionDto),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized - Buyer JWT required"),
        (status = 403, description = "Forbidden - Order belongs to another buyer"),
        (status = 404, description = "Order not found"),
        (status = 422, description = "Validation failed or order already paid")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Payments"
)]
pub async fn buyer_create_payment_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    ValidatedJson(req): ValidatedJson<CreatePaymentRequest>,
) -> Result<(StatusCode, Json<PaymentTransactionDto>), ApiError> {
    // 1. Fetch order details to verify existence and ownership
    let order = state
        .order_contract
        .get_order(req.order_id)
        .await
        .map_err(|e| match e {
            ContractError::NotFound(_) => ApiError::new(
                ErrorCode::ResourceNotFound,
                format!("Order with ID {} not found", req.order_id),
                StatusCode::NOT_FOUND,
            ),
            other => ApiError::from(other),
        })?;

    // 2. Ownership verification: if order has a buyer_id, it must match current buyer
    if let Some(buyer_id) = order.buyer_id {
        if buyer_id != claims.sub {
            return Err(ApiError::new(
                ErrorCode::InsufficientPermissions,
                "Akses ditolak: pesanan ini bukan milik akun Anda".to_string(),
                StatusCode::FORBIDDEN,
            ));
        }
    }

    // 3. Create payment transaction via PaymentContract
    let payment = state
        .payment_contract
        .create_payment(
            order.id,
            order.total_amount,
            &order.customer_name,
            &order.customer_email,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(payment)))
}

/// Webhook endpoint to receive notification callbacks from Midtrans (Public endpoint)
#[utoipa::path(
    post,
    path = "/api/v1/payments/notification",
    responses(
        (status = 200, description = "Payment notification processed successfully"),
        (status = 400, description = "Invalid payload"),
        (status = 401, description = "Invalid webhook signature"),
        (status = 404, description = "Payment transaction for order not found")
    ),
    tag = "Payments"
)]
pub async fn payment_notification_handler(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.payment_contract.handle_notification(payload).await?;

    Ok(Json(json!({
        "status": "ok",
        "message": "Payment notification processed successfully"
    })))
}

/// Retrieve payment status for a specific order (Buyer or Seller)
#[utoipa::path(
    get,
    path = "/api/v1/payments/order/{order_id}",
    params(
        ("order_id" = Uuid, Path, description = "Order identifier")
    ),
    responses(
        (status = 200, description = "Payment details", body = PaymentTransactionDto),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Not authorized to view this order payment"),
        (status = 404, description = "Payment not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Payments"
)]
pub async fn get_payment_by_order_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(order_id): Path<Uuid>,
) -> Result<Json<PaymentTransactionDto>, ApiError> {
    // If buyer, verify order ownership
    if claims.is_buyer() {
        if let Ok(order) = state.order_contract.get_order(order_id).await {
            if let Some(buyer_id) = order.buyer_id {
                if buyer_id != claims.sub {
                    return Err(ApiError::new(
                        ErrorCode::InsufficientPermissions,
                        "Akses ditolak: pesanan ini bukan milik akun Anda".to_string(),
                        StatusCode::FORBIDDEN,
                    ));
                }
            }
        }
    }

    let payment = state
        .payment_contract
        .get_payment_by_order(order_id)
        .await?
        .ok_or_else(|| {
            ApiError::new(
                ErrorCode::ResourceNotFound,
                format!("Payment transaction for order {} not found", order_id),
                StatusCode::NOT_FOUND,
            )
        })?;

    Ok(Json(payment))
}

/// Retrieve public Midtrans Snap configuration for frontend checkout
#[utoipa::path(
    get,
    path = "/api/v1/payments/config",
    responses(
        (status = 200, description = "Public Midtrans Snap configuration", body = PaymentConfigDto)
    ),
    tag = "Payments"
)]
pub async fn get_payment_config_handler(State(state): State<AppState>) -> Json<PaymentConfigDto> {
    Json(state.payment_contract.get_config())
}
