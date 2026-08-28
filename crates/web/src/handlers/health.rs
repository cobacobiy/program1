use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

use crate::state::AppState;

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

pub async fn get_store_info(State(state): State<AppState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "store_name": state.store_name,
            "currency": state.store_currency,
        })),
    )
}
