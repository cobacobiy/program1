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
use program1_web::{create_app, rate_limit::IpRateLimiter, AppState};
use serde_json::Value;
use tower::ServiceExt;

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

    let _ = user_module.seed_default_users().await;
    let _ = catalog_module.seed_default_catalog().await;
    let _ = channel_module.seed_default_channels().await;

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
        started_at: std::time::Instant::now(),
    };

    create_app(state)
}

#[tokio::test]
async fn test_liveness_health_check_returns_version_and_uptime() {
    let app = setup_test_app().await;

    let req = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(body["status"], "healthy");
    assert_eq!(body["app"], "program1");
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(body["architecture"], "modular_monolith");
    assert!(body["uptime_seconds"].is_number());
    assert!(body["timestamp"].is_string());
}

#[tokio::test]
async fn test_readiness_probe_returns_subsystem_checks() {
    let app = setup_test_app().await;

    let req = Request::builder()
        .uri("/health/ready")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(body["status"], "ready");
    assert_eq!(body["checks"]["catalog_module"], "ok");
    assert_eq!(body["checks"]["inventory_module"], "ok");
    assert_eq!(body["checks"]["user_module"], "ok");
}

#[tokio::test]
async fn test_request_id_generated_when_missing() {
    let app = setup_test_app().await;

    let req = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let request_id_hdr = res.headers().get("x-request-id");
    assert!(request_id_hdr.is_some());
    let req_id_str = request_id_hdr.unwrap().to_str().unwrap();
    assert!(!req_id_str.is_empty());
    // Should be valid UUID
    assert!(uuid::Uuid::parse_str(req_id_str).is_ok());
}

#[tokio::test]
async fn test_request_id_preserved_when_provided() {
    let app = setup_test_app().await;

    let custom_id = "trace-custom-correlation-id-98765";
    let req = Request::builder()
        .uri("/health")
        .method("GET")
        .header("x-request-id", custom_id)
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let request_id_hdr = res.headers().get("x-request-id");
    assert_eq!(request_id_hdr.unwrap().to_str().unwrap(), custom_id);
}
