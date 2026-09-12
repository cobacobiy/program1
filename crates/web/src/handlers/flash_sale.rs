use axum::{
    extract::{Path, State},
    Json,
};

use crate::error::ApiError;
use crate::state::{AppState, ValidatedJson};
use program1_contracts::{
    AddFlashSaleItemRequest, CreateFlashSaleSessionRequest, FlashSaleItemDto, FlashSaleSessionDto,
};

/// Public endpoint: Get active flash sale session with countdown and items
#[utoipa::path(
    get,
    path = "/api/v1/flash-sales/active",
    responses(
        (status = 200, description = "Active flash sale session, or null if none", body = Option<FlashSaleSessionDto>),
    ),
    tag = "Flash Sale"
)]
pub async fn get_active_flash_sale_handler(
    State(state): State<AppState>,
) -> Result<Json<Option<FlashSaleSessionDto>>, ApiError> {
    let session = state.flash_sale_contract.get_active_session().await?;
    Ok(Json(session))
}

/// Admin endpoint: List all flash sale sessions
#[utoipa::path(
    get,
    path = "/api/v1/admin/flash-sales",
    responses(
        (status = 200, description = "List of all flash sale sessions", body = Vec<FlashSaleSessionDto>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "Flash Sale"
)]
pub async fn list_flash_sale_sessions_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<FlashSaleSessionDto>>, ApiError> {
    let list = state.flash_sale_contract.list_sessions().await?;
    Ok(Json(list))
}

/// Admin endpoint: Create a new flash sale session
#[utoipa::path(
    post,
    path = "/api/v1/admin/flash-sales",
    request_body = CreateFlashSaleSessionRequest,
    responses(
        (status = 200, description = "Flash sale session created successfully", body = FlashSaleSessionDto),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "Flash Sale"
)]
pub async fn create_flash_sale_session_handler(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateFlashSaleSessionRequest>,
) -> Result<Json<FlashSaleSessionDto>, ApiError> {
    let session = state.flash_sale_contract.create_session(payload).await?;
    Ok(Json(session))
}

/// Admin endpoint: Add product to flash sale session
#[utoipa::path(
    post,
    path = "/api/v1/admin/flash-sales/{id}/items",
    params(
        ("id" = String, Path, description = "Session ID")
    ),
    request_body = AddFlashSaleItemRequest,
    responses(
        (status = 200, description = "Item added to flash sale session", body = FlashSaleItemDto),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "Flash Sale"
)]
pub async fn add_flash_sale_item_handler(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    ValidatedJson(payload): ValidatedJson<AddFlashSaleItemRequest>,
) -> Result<Json<FlashSaleItemDto>, ApiError> {
    let item = state
        .flash_sale_contract
        .add_item_to_session(&session_id, payload)
        .await?;
    Ok(Json(item))
}

/// Admin endpoint: Toggle active status of a flash sale session
#[utoipa::path(
    put,
    path = "/api/v1/admin/flash-sales/{id}/toggle",
    params(
        ("id" = String, Path, description = "Session ID")
    ),
    responses(
        (status = 200, description = "Toggled flash sale session", body = FlashSaleSessionDto),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "Flash Sale"
)]
pub async fn toggle_flash_sale_session_handler(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<FlashSaleSessionDto>, ApiError> {
    let session = state
        .flash_sale_contract
        .toggle_session(&session_id)
        .await?;
    Ok(Json(session))
}
