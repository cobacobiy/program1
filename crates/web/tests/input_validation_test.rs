use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use program1_contracts::{AuthContract, UserContract};

use program1_core::init_database;
use program1_module_analytics::AnalyticsModule;
use program1_module_auth::AuthModule;
use program1_module_catalog::CatalogModule;
use program1_module_channel::ChannelSyncModule;
use program1_module_inventory::InventoryModule;
use program1_module_order::OrderModule;
use program1_module_user::UserModule;
use program1_web::{create_app, AppState};
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

async fn setup_test_app() -> (axum::Router, String) {
    let secret = "test-jwt-secret-key-minimum-32-characters-length!".to_string();
    let pool = init_database("sqlite::memory:").await.expect("Test DB init failed");

    let user_module = Arc::new(UserModule::new(pool.clone()));
    let auth_module = Arc::new(AuthModule::new(secret.clone(), 24));
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

    let _ = user_module.seed_default_users().await;
    let _ = catalog_module.seed_default_catalog().await;
    let _ = channel_module.seed_default_channels().await;

    // Generate admin token for protected routes
    let admin_user = user_module.authenticate("admin", "admin123").await.unwrap();
    let admin_token = auth_module.generate_token(&admin_user).unwrap();

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



    let router = create_app(state);
    (router, admin_token)
}

#[tokio::test]
async fn test_invalid_email_format_returns_422() {
    let (app, _) = setup_test_app().await;

    let payload = serde_json::json!({
        "customer_name": "John Doe",
        "customer_email": "invalid-email-format-without-at",
        "shipping_address": "Jl. Merdeka No. 123",
        "items": [{
            "product_id": Uuid::new_v4().to_string(),
            "quantity": 1
        }]
    });

    let req = Request::builder()
        .uri("/api/v1/orders")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"], "validation_failed");
    let details = body["details"].as_array().unwrap();
    assert!(details.iter().any(|d| d.as_str().unwrap().contains("customer_email")));
}

#[tokio::test]
async fn test_empty_catalog_name_returns_422() {
    let (app, admin_token) = setup_test_app().await;

    let payload = serde_json::json!({
        "name": "",
        "sku": "SKU-EMPTY",
        "category": "Peripherals",
        "price": 50000.0,
        "stock": 10
    });

    let req = Request::builder()
        .uri("/api/v1/catalog")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"], "validation_failed");
    let details = body["details"].as_array().unwrap();
    assert!(details.iter().any(|d| d.as_str().unwrap().contains("name")));
}

#[tokio::test]
async fn test_negative_price_returns_422() {
    let (app, admin_token) = setup_test_app().await;

    let payload = serde_json::json!({
        "name": "Negative Price Item",
        "sku": "SKU-NEG-1",
        "category": "Peripherals",
        "price": -500.0,
        "stock": 10
    });

    let req = Request::builder()
        .uri("/api/v1/catalog")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"], "validation_failed");
}

#[tokio::test]
async fn test_invalid_username_characters_returns_422() {
    let (app, admin_token) = setup_test_app().await;

    let payload = serde_json::json!({
        "username": "user!@#invalid$",
        "full_name": "Test User",
        "role": "Staff",
        "accessible_menus": ["orders"]
    });

    let req = Request::builder()
        .uri("/api/v1/users/accounts")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"], "validation_failed");
    let details = body["details"].as_array().unwrap();
    assert!(details.iter().any(|d| d.as_str().unwrap().contains("username")));
}

#[tokio::test]
async fn test_zero_quantity_order_returns_422() {
    let (app, _) = setup_test_app().await;

    let payload = serde_json::json!({
        "customer_name": "Jane Doe",
        "customer_email": "jane@example.com",
        "shipping_address": "Jl. Sudirman No. 45",
        "items": [{
            "product_id": Uuid::new_v4().to_string(),
            "quantity": 0
        }]
    });

    let req = Request::builder()
        .uri("/api/v1/orders")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[test]
fn test_html_sanitization() {
    let malicious = "<script>alert('xss')</script><b>AURA</b> Keyboard";
    let cleaned = program1_core::strip_html(malicious);
    assert_eq!(cleaned, "alert('xss')AURA Keyboard");

    let sanitized = program1_core::sanitize_text("   <p>Some text</p>   ", 8);
    assert_eq!(sanitized, "Some tex");
}
