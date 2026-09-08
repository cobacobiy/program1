use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use program1_contracts::{JwtClaims, NotificationCountDto, NotificationDto};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct NotificationQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ==========================================
// BUYER NOTIFICATION ENDPOINTS
// ==========================================

/// List notifications for authenticated buyer
#[utoipa::path(
    get,
    path = "/api/v1/buyer/notifications",
    params(
        ("limit" = Option<i64>, Query, description = "Max results to return (default 20, max 100)"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination (default 0)")
    ),
    responses(
        (status = 200, description = "List of buyer notifications", body = Vec<NotificationDto>),
        (status = 401, description = "Unauthorized", body = ApiError)
    ),
    security(("bearer_auth" = [])),
    tag = "Buyer Notifications"
)]
pub async fn buyer_list_notifications_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Query(query): Query<NotificationQuery>,
) -> Result<Json<Vec<NotificationDto>>, ApiError> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);
    let buyer_id = claims.sub.to_string();

    let list = state
        .notification_contract
        .list_notifications("buyer", &buyer_id, limit, offset)
        .await?;

    Ok(Json(list))
}

/// Get unread and total notification counts for authenticated buyer
#[utoipa::path(
    get,
    path = "/api/v1/buyer/notifications/count",
    responses(
        (status = 200, description = "Buyer notification counts", body = NotificationCountDto),
        (status = 401, description = "Unauthorized", body = ApiError)
    ),
    security(("bearer_auth" = [])),
    tag = "Buyer Notifications"
)]
pub async fn buyer_notification_count_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
) -> Result<Json<NotificationCountDto>, ApiError> {
    let buyer_id = claims.sub.to_string();
    let counts = state
        .notification_contract
        .get_unread_count("buyer", &buyer_id)
        .await?;

    Ok(Json(counts))
}

/// Mark single buyer notification as read
#[utoipa::path(
    post,
    path = "/api/v1/buyer/notifications/{id}/read",
    params(
        ("id" = String, Path, description = "Notification ID")
    ),
    responses(
        (status = 200, description = "Notification marked as read"),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Notification not found", body = ApiError)
    ),
    security(("bearer_auth" = [])),
    tag = "Buyer Notifications"
)]
pub async fn buyer_mark_read_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let buyer_id = claims.sub.to_string();
    state
        .notification_contract
        .mark_as_read(&id, &buyer_id)
        .await?;

    Ok(Json(json!({ "success": true, "message": "Notification marked as read" })))
}

/// Mark all notifications as read for authenticated buyer
#[utoipa::path(
    post,
    path = "/api/v1/buyer/notifications/read-all",
    responses(
        (status = 200, description = "All notifications marked as read"),
        (status = 401, description = "Unauthorized", body = ApiError)
    ),
    security(("bearer_auth" = [])),
    tag = "Buyer Notifications"
)]
pub async fn buyer_mark_all_read_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
) -> Result<Json<Value>, ApiError> {
    let buyer_id = claims.sub.to_string();
    state
        .notification_contract
        .mark_all_as_read("buyer", &buyer_id)
        .await?;

    Ok(Json(json!({ "success": true, "message": "All notifications marked as read" })))
}

// ==========================================
// ADMIN / SELLER NOTIFICATION ENDPOINTS
// ==========================================

/// List notifications for admin/seller
#[utoipa::path(
    get,
    path = "/api/v1/admin/notifications",
    params(
        ("limit" = Option<i64>, Query, description = "Max results to return (default 20, max 100)"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination (default 0)")
    ),
    responses(
        (status = 200, description = "List of admin notifications", body = Vec<NotificationDto>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 403, description = "Forbidden", body = ApiError)
    ),
    security(("bearer_auth" = [])),
    tag = "Admin Notifications"
)]
pub async fn admin_list_notifications_handler(
    State(state): State<AppState>,
    Query(query): Query<NotificationQuery>,
) -> Result<Json<Vec<NotificationDto>>, ApiError> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);

    let list = state
        .notification_contract
        .list_notifications("seller", "admin", limit, offset)
        .await?;

    Ok(Json(list))
}

/// Get unread and total notification counts for admin/seller
#[utoipa::path(
    get,
    path = "/api/v1/admin/notifications/count",
    responses(
        (status = 200, description = "Admin notification counts", body = NotificationCountDto),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 403, description = "Forbidden", body = ApiError)
    ),
    security(("bearer_auth" = [])),
    tag = "Admin Notifications"
)]
pub async fn admin_notification_count_handler(
    State(state): State<AppState>,
) -> Result<Json<NotificationCountDto>, ApiError> {
    let counts = state
        .notification_contract
        .get_unread_count("seller", "admin")
        .await?;

    Ok(Json(counts))
}

/// Mark single admin notification as read
#[utoipa::path(
    post,
    path = "/api/v1/admin/notifications/{id}/read",
    params(
        ("id" = String, Path, description = "Notification ID")
    ),
    responses(
        (status = 200, description = "Notification marked as read"),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Notification not found", body = ApiError)
    ),
    security(("bearer_auth" = [])),
    tag = "Admin Notifications"
)]
pub async fn admin_mark_read_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    state
        .notification_contract
        .mark_as_read(&id, "admin")
        .await?;

    Ok(Json(json!({ "success": true, "message": "Notification marked as read" })))
}

/// Mark all notifications as read for admin/seller
#[utoipa::path(
    post,
    path = "/api/v1/admin/notifications/read-all",
    responses(
        (status = 200, description = "All notifications marked as read"),
        (status = 401, description = "Unauthorized", body = ApiError)
    ),
    security(("bearer_auth" = [])),
    tag = "Admin Notifications"
)]
pub async fn admin_mark_all_read_handler(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    state
        .notification_contract
        .mark_all_as_read("seller", "admin")
        .await?;

    Ok(Json(json!({ "success": true, "message": "All notifications marked as read" })))
}
