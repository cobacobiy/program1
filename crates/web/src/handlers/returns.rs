use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use program1_contracts::{
    CreateReturnRequest, JwtClaims, ProcessReturnRequest, ReturnRequestDto,
    UpdateReturnStatusRequest,
};
use serde::Deserialize;

use crate::error::ApiError;
use crate::state::{AppState, ValidatedJson};

#[derive(Debug, Deserialize)]
pub struct ReturnListQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ==========================================
// BUYER RETURN ENDPOINTS
// ==========================================

/// Submit a return request for a delivered order
#[utoipa::path(
    post,
    path = "/api/v1/buyer/returns",
    request_body = CreateReturnRequest,
    responses(
        (status = 201, description = "Return request submitted", body = ReturnRequestDto),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 409, description = "Return request already exists", body = ApiError)
    ),
    security(("bearer_auth" = [])),
    tag = "Buyer Returns"
)]
pub async fn buyer_create_return_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    ValidatedJson(payload): ValidatedJson<CreateReturnRequest>,
) -> Result<(StatusCode, Json<ReturnRequestDto>), ApiError> {
    let buyer_id = claims.sub.to_string();
    let res = state
        .return_contract
        .create_return_request(&buyer_id, payload)
        .await?;

    Ok((StatusCode::CREATED, Json(res)))
}

/// List return requests submitted by authenticated buyer
#[utoipa::path(
    get,
    path = "/api/v1/buyer/returns",
    responses(
        (status = 200, description = "List of buyer return requests", body = Vec<ReturnRequestDto>),
        (status = 401, description = "Unauthorized", body = ApiError)
    ),
    security(("bearer_auth" = [])),
    tag = "Buyer Returns"
)]
pub async fn buyer_list_returns_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
) -> Result<Json<Vec<ReturnRequestDto>>, ApiError> {
    let buyer_id = claims.sub.to_string();
    let list = state.return_contract.list_buyer_returns(&buyer_id).await?;
    Ok(Json(list))
}

/// Get return request by order_id for buyer
#[utoipa::path(
    get,
    path = "/api/v1/buyer/returns/order/{order_id}",
    params(
        ("order_id" = String, Path, description = "Order ID")
    ),
    responses(
        (status = 200, description = "Return request detail or null", body = Option<ReturnRequestDto>),
        (status = 401, description = "Unauthorized", body = ApiError)
    ),
    security(("bearer_auth" = [])),
    tag = "Buyer Returns"
)]
pub async fn buyer_get_return_by_order_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(order_id): Path<String>,
) -> Result<Json<Option<ReturnRequestDto>>, ApiError> {
    let buyer_id = claims.sub.to_string();
    let ret = state.return_contract.get_return_by_order(&order_id).await?;
    
    // Check ownership if return request exists
    if let Some(r) = &ret {
        if r.buyer_id != buyer_id {
            return Err(ApiError::new(
                program1_contracts::ErrorCode::InsufficientPermissions,
                "Anda tidak memiliki akses ke data retur ini",
                axum::http::StatusCode::FORBIDDEN,
            ));
        }
    }

    Ok(Json(ret))
}

// ==========================================
// ADMIN RETURN ENDPOINTS
// ==========================================

/// List all return requests for admin/seller
#[utoipa::path(
    get,
    path = "/api/v1/admin/returns",
    params(
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("limit" = Option<i64>, Query, description = "Limit max items"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses(
        (status = 200, description = "List of return requests", body = Vec<ReturnRequestDto>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 403, description = "Forbidden", body = ApiError)
    ),
    security(("bearer_auth" = [])),
    tag = "Admin Returns"
)]
pub async fn admin_list_returns_handler(
    State(state): State<AppState>,
    Query(query): Query<ReturnListQuery>,
) -> Result<Json<Vec<ReturnRequestDto>>, ApiError> {
    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);
    let status_filter = query.status.as_deref();

    let list = state
        .return_contract
        .list_all_returns(status_filter, limit, offset)
        .await?;

    Ok(Json(list))
}

/// Process (approve or reject) a return request
#[utoipa::path(
    post,
    path = "/api/v1/admin/returns/{id}/process",
    params(
        ("id" = String, Path, description = "Return request ID")
    ),
    request_body = ProcessReturnRequest,
    responses(
        (status = 200, description = "Return request processed", body = ReturnRequestDto),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Return request not found", body = ApiError)
    ),
    security(("bearer_auth" = [])),
    tag = "Admin Returns"
)]
pub async fn admin_process_return_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<String>,
    ValidatedJson(payload): ValidatedJson<ProcessReturnRequest>,
) -> Result<Json<ReturnRequestDto>, ApiError> {
    let admin_id = claims.username.clone();
    let res = state
        .return_contract
        .process_return(&id, &admin_id, payload)
        .await?;

    Ok(Json(res))
}

/// Update lifecycle status of a return request (e.g. return_shipped, received, refunded)
#[utoipa::path(
    post,
    path = "/api/v1/admin/returns/{id}/status",
    params(
        ("id" = String, Path, description = "Return request ID")
    ),
    request_body = UpdateReturnStatusRequest,
    responses(
        (status = 200, description = "Return status updated", body = ReturnRequestDto),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Return request not found", body = ApiError)
    ),
    security(("bearer_auth" = [])),
    tag = "Admin Returns"
)]
pub async fn admin_update_return_status_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<String>,
    ValidatedJson(payload): ValidatedJson<UpdateReturnStatusRequest>,
) -> Result<Json<ReturnRequestDto>, ApiError> {
    let admin_id = claims.username.clone();
    let res = state
        .return_contract
        .update_return_status(&id, &admin_id, payload)
        .await?;

    Ok(Json(res))
}

/// Update return status by buyer (e.g. mark as return_shipped)
#[utoipa::path(
    put,
    path = "/api/v1/buyer/returns/{id}/status",
    params(
        ("id" = String, Path, description = "Return request ID")
    ),
    request_body = UpdateReturnStatusRequest,
    responses(
        (status = 200, description = "Return status updated", body = ReturnRequestDto),
        (status = 400, description = "Validation error", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Return request not found", body = ApiError)
    ),
    security(("bearer_auth" = [])),
    tag = "Buyer Returns"
)]
pub async fn buyer_update_return_status_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(id): Path<String>,
    ValidatedJson(payload): ValidatedJson<UpdateReturnStatusRequest>,
) -> Result<Json<ReturnRequestDto>, ApiError> {
    let buyer_id = claims.sub.to_string();
    let res = state
        .return_contract
        .update_return_status(&id, &buyer_id, payload)
        .await?;

    Ok(Json(res))
}
