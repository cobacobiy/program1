use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use program1_contracts::{AuthContract, UserContract};
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
use uuid::Uuid;

async fn setup_test_app() -> (axum::Router, String) {
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
        rate_limiter,
    };

    (create_app(state), admin_token)
}

#[tokio::test]
async fn test_resource_not_found_standard_error_format() {
    let (app, _) = setup_test_app().await;

    let non_existent_id = Uuid::new_v4();
    let req = Request::builder()
        .uri(format!("/api/v1/catalog/{}", non_existent_id))
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();

    assert!(body.get("error").is_some());
    let err = &body["error"];
    assert_eq!(err["code"], "RESOURCE_NOT_FOUND");
    assert_eq!(err["status"], 404);
    assert!(!err["message"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn test_validation_failed_standard_error_format() {
    let (app, admin_token) = setup_test_app().await;

    let invalid_catalog_payload = serde_json::json!({
        "name": "",
        "sku": "SKU-EMPTY",
        "category": "Peripherals",
        "price": -100.0,
        "stock": 10
    });

    let req = Request::builder()
        .uri("/api/v1/catalog")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&invalid_catalog_payload).unwrap()))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();

    assert!(body.get("error").is_some());
    let err = &body["error"];
    assert_eq!(err["code"], "VALIDATION_FAILED");
    assert_eq!(err["status"], 422);
    assert!(err.get("details").is_some());
    let details = err["details"].as_array().unwrap();
    assert!(details.len() >= 2);
}

#[tokio::test]
async fn test_insufficient_stock_error_format() {
    let (app, _) = setup_test_app().await;

    // Request order with quantity far exceeding catalog stock
    let order_payload = serde_json::json!({
        "customer_name": "Greedy Buyer",
        "customer_email": "buyer@example.com",
        "shipping_address": "Jl. Borong Banyak 99",
        "items": [{
            "product_id": "10000000-0000-0000-0000-000000000001",
            "quantity": 9999
        }]
    });

    let req = Request::builder()
        .uri("/api/v1/orders")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&order_payload).unwrap()))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();

    assert!(body.get("error").is_some());
    let err = &body["error"];
    assert_eq!(err["code"], "INSUFFICIENT_STOCK");
    assert_eq!(err["status"], 409);
    assert!(err["message"].as_str().unwrap().contains("Insufficient stock"));
}
