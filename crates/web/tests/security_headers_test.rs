use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use program1_core::init_database;
use program1_module_analytics::AnalyticsModule;
use program1_module_auth::AuthModule;
use program1_module_catalog::CatalogModule;
use program1_module_channel::ChannelSyncModule;
use program1_module_inventory::InventoryModule;
use program1_module_order::OrderModule;
use program1_module_user::UserModule;
use program1_web::{create_app, AppState};
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
    let audit_module = Arc::new(program1_module_audit::AuditModule::new(pool.clone()));

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
        rate_limiter: Arc::new(program1_web::rate_limit::IpRateLimiter::new()),
    };



    create_app(state)
}

#[tokio::test]
async fn test_security_headers_present() {
    let app = setup_test_app().await;

    let req = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let headers = response.headers();
    assert_eq!(
        headers.get(header::X_FRAME_OPTIONS).unwrap().to_str().unwrap(),
        "DENY"
    );
    assert_eq!(
        headers.get(header::X_CONTENT_TYPE_OPTIONS).unwrap().to_str().unwrap(),
        "nosniff"
    );
    assert_eq!(
        headers.get("x-xss-protection").unwrap().to_str().unwrap(),
        "1; mode=block"
    );
    assert_eq!(
        headers.get(header::REFERRER_POLICY).unwrap().to_str().unwrap(),
        "strict-origin-when-cross-origin"
    );
    assert!(headers.get(header::CONTENT_SECURITY_POLICY).is_some());
}

#[tokio::test]
async fn test_cors_preflight_allowed_origin() {
    let app = setup_test_app().await;

    let req = Request::builder()
        .uri("/api/v1/catalog")
        .method("OPTIONS")
        .header(header::ORIGIN, "http://localhost:3000")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "authorization,content-type")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let headers = response.headers();
    assert_eq!(
        headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap().to_str().unwrap(),
        "http://localhost:3000"
    );
    assert_eq!(
        headers.get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS).unwrap().to_str().unwrap(),
        "true"
    );
}

#[tokio::test]
async fn test_cors_disallowed_origin() {
    let app = setup_test_app().await;

    let req = Request::builder()
        .uri("/api/v1/catalog")
        .method("OPTIONS")
        .header(header::ORIGIN, "http://untrusted-malicious-domain.com")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    let headers = response.headers();
    assert!(headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none());
}
