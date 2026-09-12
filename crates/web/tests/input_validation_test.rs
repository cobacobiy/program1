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

async fn setup_test_app() -> (axum::Router, String, String, Uuid) {
    let secret = "test-jwt-secret-key-minimum-32-characters-length!".to_string();
    let pool = init_database("sqlite::memory:")
        .await
        .expect("Test DB init failed");

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

    let google_verifier = Arc::new(program1_module_buyer::ProductionGoogleVerifier {
        client_id: "test".to_string(),
    });
    let sms_sender = Arc::new(program1_module_buyer::ConsoleOrProviderSmsSender::default());
    let buyer_module = Arc::new(program1_module_buyer::BuyerModule::new(
        pool.clone(),
        auth_module.clone(),
        google_verifier,
        sms_sender,
        audit_module.clone(),
    ));

    let _ = user_module.seed_default_users().await;
    let _ = catalog_module.seed_default_catalog().await;
    let _ = channel_module.seed_default_channels().await;

    // Generate admin token for protected routes
    let admin_user = user_module.authenticate("admin", "admin123").await.unwrap();
    let admin_token = auth_module.generate_token(&admin_user).unwrap();

    // Create verified buyer and address
    let buyer_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let now_str = now.to_rfc3339();
    sqlx::query(
        "INSERT INTO buyer_accounts (id, google_sub, email, full_name, avatar_url, phone_number, phone_verified, is_active, created_at, updated_at)
         VALUES ($1, 'sub_val', 'val@buyer.com', 'Validation Buyer', NULL, '+628123456789', 1, 1, $2, $3)",
    )
    .bind(buyer_id.to_string())
    .bind(&now_str)
    .bind(&now_str)
    .execute(&pool)
    .await
    .unwrap();

    let buyer_dto = program1_contracts::BuyerAccountDto {
        id: buyer_id,
        google_sub: Some("sub_val".to_string()),
        email: "val@buyer.com".to_string(),
        full_name: "Validation Buyer".to_string(),
        avatar_url: None,
        phone_number: Some("+628123456789".to_string()),
        phone_verified: true,
        is_active: true,
        created_at: now,
        updated_at: now,
    };
    let buyer_token = auth_module.generate_buyer_token(&buyer_dto).unwrap();

    let addr_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO buyer_addresses (id, buyer_id, recipient_name, phone_number, street_address, subdistrict, city, province, postal_code, is_default, created_at, updated_at)
         VALUES ($1, $2, 'Val Buyer', '+628123456789', 'Jl. Val 1', 'Sub', 'City', 'Prov', '12345', 1, $3, $4)",
    )
    .bind(addr_id.to_string())
    .bind(buyer_id.to_string())
    .bind(&now_str)
    .bind(&now_str)
    .execute(&pool)
    .await
    .unwrap();

    let chat_module = Arc::new(program1_module_chat::ChatModule::new(pool.clone()));
    let payment_module = Arc::new(program1_module_payment::PaymentModule::new(
        pool.clone(),
        "test-server-key".to_string(),
        "test-client-key".to_string(),
        false,
    ));
    let coupon_module = Arc::new(program1_module_coupon::CouponModule::new(pool.clone()));
    let review_module = Arc::new(program1_module_review::ReviewModule::new(pool.clone()));

    let state = AppState {
        store_name: "Test Store".to_string(),
        store_currency: "IDR".to_string(),
        store_whatsapp_number: "085810007735".to_string(),
        user_contract: user_module,
        auth_contract: auth_module,
        catalog_contract: catalog_module,
        inventory_contract: inventory_module,
        channel_contract: channel_module,
        order_contract: order_module,
        analytics_contract: analytics_module,
        audit_contract: audit_module,
        buyer_contract: buyer_module,
        chat_contract: chat_module,
        payment_contract: payment_module,
        coupon_contract: coupon_module,
        review_contract: review_module,
        shipping_contract: Arc::new(program1_module_shipping::ShippingModule::new("".to_string(), "starter".to_string(), "152".to_string())),
        notification_contract: Arc::new(program1_module_notification::NotificationModule::new(pool.clone())),
        return_contract: Arc::new(program1_module_return::ReturnModule::new(pool.clone())),
        backup_contract: Arc::new(program1_core::backup::BackupService::new(pool.clone(), "./target/test_backups")),
        rate_limiter,
        started_at: std::time::Instant::now(),
        google_client_id: "test".to_string(),
    };

    let router = create_app(state);
    (router, admin_token, buyer_token, addr_id)
}

#[tokio::test]
async fn test_empty_order_items_returns_422() {
    let (app, _, buyer_token, addr_id) = setup_test_app().await;

    let payload = serde_json::json!({
        "address_id": addr_id,
        "items": []
    });

    let req = Request::builder()
        .uri("/api/v1/orders")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    let err = &body["error"];
    assert_eq!(err["code"], "VALIDATION_FAILED");
    let details = err["details"].as_array().unwrap();
    assert!(details
        .iter()
        .any(|d| d.as_str().unwrap().contains("items")));
}

#[tokio::test]
async fn test_empty_catalog_name_returns_422() {
    let (app, admin_token, _, _) = setup_test_app().await;

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
    let err = &body["error"];
    assert_eq!(err["code"], "VALIDATION_FAILED");
    let details = err["details"].as_array().unwrap();
    assert!(details.iter().any(|d| d.as_str().unwrap().contains("name")));
}

#[tokio::test]
async fn test_negative_price_returns_422() {
    let (app, admin_token, _, _) = setup_test_app().await;

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
    let err = &body["error"];
    assert_eq!(err["code"], "VALIDATION_FAILED");
}

#[tokio::test]
async fn test_invalid_username_characters_returns_422() {
    let (app, admin_token, _, _) = setup_test_app().await;

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
    let err = &body["error"];
    assert_eq!(err["code"], "VALIDATION_FAILED");
    let details = err["details"].as_array().unwrap();
    assert!(details
        .iter()
        .any(|d| d.as_str().unwrap().contains("username")));
}

#[tokio::test]
async fn test_zero_quantity_order_returns_422() {
    let (app, _, buyer_token, addr_id) = setup_test_app().await;

    let payload = serde_json::json!({
        "address_id": addr_id,
        "items": [{
            "product_id": Uuid::new_v4().to_string(),
            "quantity": 0
        }]
    });

    let req = Request::builder()
        .uri("/api/v1/orders")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
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
