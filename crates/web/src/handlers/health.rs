use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

use crate::state::AppState;

/// Check service health status
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is operational", body = serde_json::Value)
    ),
    tag = "Health"
)]
pub async fn health_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "status": "healthy",
            "app": "program1",
            "architecture": "modular_monolith",
            "version": "0.1.0"
        })),
    )
}

/// Retrieve storefront metadata and operating currency
#[utoipa::path(
    get,
    path = "/api/v1/store/info",
    responses(
        (status = 200, description = "Store information retrieved", body = serde_json::Value)
    ),
    tag = "Health"
)]
pub async fn get_store_info(State(state): State<AppState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "store_name": state.store_name,
            "currency": state.store_currency,
        })),
    )
}
