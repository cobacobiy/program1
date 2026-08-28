use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use program1_core::init_database;
use program1_module_analytics::AnalyticsModule;
use program1_module_audit::AuditModule;
use program1_module_auth::AuthModule;
use program1_module_catalog::CatalogModule;
use program1_module_channel::ChannelSyncModule;
use program1_module_inventory::InventoryModule;
use program1_module_order::OrderModule;
use program1_module_user::UserModule;
use program1_web::{create_app, rate_limit::IpRateLimiter, ApiDoc, AppState};
use serde_json::Value;
use tower::ServiceExt;
use utoipa::OpenApi;

async fn setup_test_app() -> axum::Router {
    let secret = "test-jwt-secret-key-minimum-32-characters-length!".to_string();
    let pool = init_database("sqlite::memory:").await.expect("Test DB init failed");

    let user_module = Arc::new(UserModule::new(pool.clone()));
    let auth_module = Arc::new(AuthModule::new(secret, 24));
    let catalog_module = Arc::new(CatalogModule::new(pool.clone()));
    let inventory_module = Arc::new(InventoryModule::new(pool.clone(), catalog_module.clone()));
    let channel_module = Arc::new(ChannelSyncModule::new(pool.clone()));
    let order_module = Arc::new(OrderModule::new(
        pool.clone(),
        catalog_module.clone(),
        inventory_module.clone(),
    ));
    let analytics_module = Arc::new(AnalyticsModule::new(
        catalog_module.clone(),
        order_module.clone(),
    ));
    let audit_module = Arc::new(AuditModule::new(pool.clone()));
    let rate_limiter = Arc::new(IpRateLimiter::new());

    let state = AppState {
        store_name: "Test Store".to_string(),
        store_currency: "IDR".to_string(),
        user_contract: user_module,
        auth_contract: auth_module,
        catalog_contract: catalog_module,
        inventory_contract: inventory_module,
        channel_contract: channel_module,
        order_contract: order_module,
        analytics_contract: analytics_module,
        audit_contract: audit_module,
        rate_limiter,
    };

    create_app(state)
}

#[tokio::test]
async fn test_openapi_spec_structure() {
    let spec = ApiDoc::openapi();
    let json_str = serde_json::to_string_pretty(&spec).unwrap();
    assert!(json_str.contains("Program1"));
    assert!(json_str.contains("1.0.0"));
    assert!(json_str.contains("/api/v1/catalog"));
    assert!(json_str.contains("/api/v1/orders"));
    assert!(json_str.contains("/api/v1/auth/login"));
    assert!(json_str.contains("/api/v1/audit/logs"));
}

#[tokio::test]
async fn test_openapi_endpoint_returns_json() {
    let app = setup_test_app().await;

    let req = Request::builder()
        .uri("/api-doc/openapi.json")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let spec: Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(spec["info"]["version"], "1.0.0");
    assert!(spec["paths"]["/api/v1/catalog"].is_object());
    assert!(spec["components"]["schemas"]["CatalogItemDto"].is_object());
}

#[tokio::test]
async fn test_x_api_version_header_returned() {
    let app = setup_test_app().await;

    let req = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get("x-api-version").unwrap().to_str().unwrap(),
        "1.0.0"
    );
}
