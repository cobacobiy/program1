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


async fn setup_test_app() -> (axum::Router, String, String, Arc<AuditModule>) {
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

    // Tokens
    let admin_user = user_module.authenticate("admin", "admin123").await.unwrap();
    let admin_token = auth_module.generate_token(&admin_user).unwrap();

    let staff_user = user_module.authenticate("staff_cs", "admin123").await.unwrap();
    let staff_token = auth_module.generate_token(&staff_user).unwrap();


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
        audit_contract: audit_module.clone(),
        rate_limiter,
    };

    (create_app(state), admin_token, staff_token, audit_module)
}

#[tokio::test]
async fn test_login_and_order_create_audit_logs() {
    let (app, admin_token, _, _) = setup_test_app().await;

    // 1. Perform a failed login attempt
    let fail_login_payload = serde_json::json!({
        "username": "unknown_hacker",
        "password": "wrongpassword123"
    });
    let req = Request::builder()
        .uri("/api/v1/auth/login")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&fail_login_payload).unwrap()))
        .unwrap();
    let _ = app.clone().oneshot(req).await.unwrap();

    // 2. Perform a successful login attempt
    let success_login_payload = serde_json::json!({
        "username": "admin",
        "password": "admin123"
    });
    let req = Request::builder()
        .uri("/api/v1/auth/login")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&success_login_payload).unwrap()))
        .unwrap();
    let _ = app.clone().oneshot(req).await.unwrap();

    // 3. Place an order
    let order_payload = serde_json::json!({
        "customer_name": "Audit Test Customer",
        "customer_email": "audit@customer.com",
        "shipping_address": "Jl. Audit Trail 123",
        "items": [{
            "product_id": "10000000-0000-0000-0000-000000000001",
            "quantity": 1
        }]
    });
    let req = Request::builder()
        .uri("/api/v1/orders")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&order_payload).unwrap()))
        .unwrap();
    let order_res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(order_res.status(), StatusCode::CREATED);

    // 4. Admin queries audit logs
    let req = Request::builder()
        .uri("/api/v1/audit/logs?limit=50")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let logs: Vec<Value> = serde_json::from_slice(&bytes).unwrap();
    assert!(logs.len() >= 3);

    // Verify actions exist in audit logs
    let actions: Vec<&str> = logs.iter().map(|l| l["action"].as_str().unwrap()).collect();
    assert!(actions.contains(&"LOGIN_FAILED"));
    assert!(actions.contains(&"LOGIN_SUCCESS"));
    assert!(actions.contains(&"ORDER_CREATED"));
}

#[tokio::test]
async fn test_audit_logs_resource_type_filter() {
    let (app, admin_token, _, _) = setup_test_app().await;

    // Place an order
    let order_payload = serde_json::json!({
        "customer_name": "Resource Filter Customer",
        "customer_email": "filter@customer.com",
        "shipping_address": "Jl. Resource 456",
        "items": [{
            "product_id": "10000000-0000-0000-0000-000000000001",
            "quantity": 1
        }]
    });
    let req = Request::builder()
        .uri("/api/v1/orders")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&order_payload).unwrap()))
        .unwrap();
    let _ = app.clone().oneshot(req).await.unwrap();

    // Query audit logs filtered by resource_type=order
    let req = Request::builder()
        .uri("/api/v1/audit/logs?resource_type=order")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let logs: Vec<Value> = serde_json::from_slice(&bytes).unwrap();
    assert!(!logs.is_empty());
    for log in logs {
        assert_eq!(log["resource_type"], "order");
    }
}

#[tokio::test]
async fn test_non_admin_cannot_access_audit_logs() {
    let (app, _, staff_token, _) = setup_test_app().await;

    // 1. Unauthenticated request -> 401
    let req = Request::builder()
        .uri("/api/v1/audit/logs")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 2. Staff user request -> 403 Forbidden
    let req = Request::builder()
        .uri("/api/v1/audit/logs")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", staff_token))
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}
