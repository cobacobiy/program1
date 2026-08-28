use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

use crate::state::AppState;

/// Check service liveness status and uptime
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is alive and running", body = serde_json::Value)
    ),
    tag = "Health"
)]
pub async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "status": "healthy",
            "app": "program1",
            "version": env!("CARGO_PKG_VERSION"),
            "architecture": "modular_monolith",
            "uptime_seconds": state.started_at.elapsed().as_secs(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    )
}

/// Check service readiness and domain dependency availability
#[utoipa::path(
    get,
    path = "/health/ready",
    responses(
        (status = 200, description = "All system dependencies are healthy", body = serde_json::Value),
        (status = 503, description = "One or more subsystems are degraded", body = serde_json::Value)
    ),
    tag = "Health"
)]
pub async fn readiness_check(State(state): State<AppState>) -> impl IntoResponse {
    let mut checks = serde_json::Map::new();
    let mut overall_healthy = true;

    // 1. Check catalog subsystem
    let catalog_ok = state.catalog_contract.list_items().await.is_ok();
    checks.insert("catalog_module".into(), json!(if catalog_ok { "ok" } else { "fail" }));
    if !catalog_ok {
        overall_healthy = false;
    }

    // 2. Check inventory subsystem
    let inventory_ok = state.inventory_contract.get_all_stocks().await.is_ok();
    checks.insert("inventory_module".into(), json!(if inventory_ok { "ok" } else { "fail" }));
    if !inventory_ok {
        overall_healthy = false;
    }

    // 3. Check user subsystem
    let user_ok = state.user_contract.list_accounts().await.is_ok();
    checks.insert("user_module".into(), json!(if user_ok { "ok" } else { "fail" }));
    if !user_ok {
        overall_healthy = false;
    }

    let status_code = if overall_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status_code,
        Json(json!({
            "status": if overall_healthy { "ready" } else { "degraded" },
            "app": "program1",
            "version": env!("CARGO_PKG_VERSION"),
            "uptime_seconds": state.started_at.elapsed().as_secs(),
            "checks": checks,
            "timestamp": chrono::Utc::now().to_rfc3339()
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
