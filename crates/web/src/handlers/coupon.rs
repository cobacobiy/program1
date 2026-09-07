use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use utoipa::ToSchema;

use crate::error::ApiError;
use crate::state::{AppState, ValidatedJson};
use program1_contracts::{
    AuditLogEntry, CouponDto, CreateCouponRequest, ValidateCouponRequest, CouponValidationResult,
};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ToggleCouponStatusRequest {
    pub is_active: bool,
}

/// List all coupons (Admin)
#[utoipa::path(
    get,
    path = "/api/v1/admin/coupons",
    responses(
        (status = 200, description = "List of all coupons", body = Vec<CouponDto>),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Coupons"
)]
pub async fn list_coupons_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<CouponDto>>, ApiError> {
    let coupons = state.coupon_contract.list_coupons().await?;
    Ok(Json(coupons))
}

/// Create a new promotional coupon (Admin)
#[utoipa::path(
    post,
    path = "/api/v1/admin/coupons",
    request_body = CreateCouponRequest,
    responses(
        (status = 201, description = "Coupon created successfully", body = CouponDto),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Coupons"
)]
pub async fn create_coupon_handler(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<CreateCouponRequest>,
) -> Result<(StatusCode, Json<CouponDto>), ApiError> {
    let coupon = state.coupon_contract.create_coupon(req).await?;

    let _ = state
        .audit_contract
        .log_action(AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor_id: None,
            actor_username: "admin".to_string(),
            action: "COUPON_CREATED".to_string(),
            resource_type: "coupon".to_string(),
            resource_id: Some(coupon.id),
            details: json!({
                "code": coupon.code,
                "discount_type": coupon.discount_type.as_str(),
                "discount_value": coupon.discount_value
            })
            .to_string(),
            ip_address: None,
        })
        .await;

    Ok((StatusCode::CREATED, Json(coupon)))
}

/// Toggle coupon active status (Admin)
#[utoipa::path(
    patch,
    path = "/api/v1/admin/coupons/{id}/status",
    params(
        ("id" = Uuid, Path, description = "Coupon identifier")
    ),
    request_body = ToggleCouponStatusRequest,
    responses(
        (status = 200, description = "Coupon status updated", body = CouponDto),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Coupon not found", body = ApiError)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Coupons"
)]
pub async fn toggle_coupon_handler(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<ToggleCouponStatusRequest>,
) -> Result<Json<CouponDto>, ApiError> {
    let coupon = state
        .coupon_contract
        .toggle_coupon_active(id, payload.is_active)
        .await?;

    let _ = state
        .audit_contract
        .log_action(AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor_id: None,
            actor_username: "admin".to_string(),
            action: "COUPON_STATUS_CHANGED".to_string(),
            resource_type: "coupon".to_string(),
            resource_id: Some(id),
            details: json!({
                "code": coupon.code,
                "is_active": payload.is_active
            })
            .to_string(),
            ip_address: None,
        })
        .await;

    Ok(Json(coupon))
}

/// Delete a coupon (Admin)
#[utoipa::path(
    delete,
    path = "/api/v1/admin/coupons/{id}",
    params(
        ("id" = Uuid, Path, description = "Coupon identifier")
    ),
    responses(
        (status = 204, description = "Coupon deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Coupon not found", body = ApiError)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Coupons"
)]
pub async fn delete_coupon_handler(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<StatusCode, ApiError> {
    state.coupon_contract.delete_coupon(id).await?;

    let _ = state
        .audit_contract
        .log_action(AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor_id: None,
            actor_username: "admin".to_string(),
            action: "COUPON_DELETED".to_string(),
            resource_type: "coupon".to_string(),
            resource_id: Some(id),
            details: json!({ "id": id }).to_string(),
            ip_address: None,
        })
        .await;

    Ok(StatusCode::NO_CONTENT)
}

/// Validate promo code against order subtotal (Public / Buyer)
#[utoipa::path(
    post,
    path = "/api/v1/coupons/validate",
    request_body = ValidateCouponRequest,
    responses(
        (status = 200, description = "Coupon validation result", body = CouponValidationResult),
        (status = 400, description = "Validation error", body = ApiError)
    ),
    tag = "Coupons"
)]
pub async fn validate_coupon_handler(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<ValidateCouponRequest>,
) -> Result<Json<CouponValidationResult>, ApiError> {
    let result = state.coupon_contract.validate_coupon(req).await?;
    Ok(Json(result))
}
