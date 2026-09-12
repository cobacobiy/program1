use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use program1_contracts::{CouponContract, JwtClaims};
use program1_core::init_database;
use program1_module_analytics::AnalyticsModule;
use program1_module_auth::AuthModule;
use program1_module_catalog::CatalogModule;
use program1_module_channel::ChannelSyncModule;
use program1_module_coupon::CouponModule;
use program1_module_inventory::InventoryModule;
use program1_module_order::OrderModule;
use program1_module_user::UserModule;
use program1_web::{create_app, AppState};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

async fn setup_test_app() -> (axum::Router, String, Arc<CouponModule>) {
    let secret = "test-jwt-secret-key-minimum-32-characters-length!".to_string();
    let pool = init_database("sqlite::memory:")
        .await
        .expect("Test DB init failed");

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
    let chat_module = Arc::new(program1_module_chat::ChatModule::new(pool.clone()));
    let payment_module = Arc::new(program1_module_payment::PaymentModule::new(
        pool.clone(),
        "test-server-key".to_string(),
        "test-client-key".to_string(),
        false,
    ));
    let coupon_module = Arc::new(CouponModule::new(pool.clone()));
    let review_module = Arc::new(program1_module_review::ReviewModule::new(pool.clone()));

    let _ = user_module.seed_default_users().await;
    let _ = catalog_module.seed_default_catalog().await;
    let _ = channel_module.seed_default_channels().await;

    let state = AppState {
        store_name: "Aura Test Store".to_string(),
        store_currency: "IDR".to_string(),
        store_whatsapp_number: "08123456789".to_string(),
        user_contract: user_module,
        auth_contract: auth_module.clone(),
        catalog_contract: catalog_module,
        inventory_contract: inventory_module,
        channel_contract: channel_module,
        order_contract: order_module,
        analytics_contract: analytics_module,
        audit_contract: audit_module,
        buyer_contract: buyer_module,
        chat_contract: chat_module,
        payment_contract: payment_module,
        coupon_contract: coupon_module.clone(),
        review_contract: review_module,
        shipping_contract: Arc::new(program1_module_shipping::ShippingModule::new("".to_string(), "starter".to_string(), "152".to_string())),
        notification_contract: Arc::new(program1_module_notification::NotificationModule::new(pool.clone())),
        return_contract: Arc::new(program1_module_return::ReturnModule::new(pool.clone())),
        backup_contract: Arc::new(program1_core::backup::BackupService::new(pool.clone(), "./target/test_backups")),
        rate_limiter: Arc::new(program1_web::rate_limit::IpRateLimiter::new()),
        started_at: std::time::Instant::now(),
        google_client_id: "test".to_string(),
    };

    let app = create_app(state);

    let seller_claims = JwtClaims {
        sub: Uuid::new_v4(),
        username: "admin_seller".to_string(),
        role: "Super Admin".to_string(),
        accessible_menus: vec!["all".to_string()],
        exp: (Utc::now() + Duration::hours(1)).timestamp(),
        iat: Utc::now().timestamp(),
        user_type: "seller_staff".to_string(),
    };
    let token = encode(
        &Header::default(),
        &seller_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("Failed to encode seller test token");

    (app, token, coupon_module)
}

#[tokio::test]
async fn test_create_and_validate_percentage_coupon() {
    let (app, token, _) = setup_test_app().await;

    // 1. Admin creates coupon
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/coupons")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "code": "diskon20",
                "description": "Diskon 20% max 50rb",
                "discount_type": "percentage",
                "discount_value": 20.0,
                "min_order_amount": 100000.0,
                "max_discount_amount": 50000.0,
                "usage_limit": 10,
                "valid_until": (Utc::now() + Duration::days(7)).to_rfc3339()
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let c: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(c["code"], "DISKON20");

    // 2. Buyer validates with 200_000 (20% is 40_000)
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/coupons/validate")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "code": "DISKON20",
                "order_amount": 200000.0
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val["is_valid"], true);
    assert_eq!(val["discount_amount"], 40000.0);
    assert_eq!(val["final_amount"], 160000.0);

    // 3. Buyer validates with 500_000 (20% is 100_000 -> capped at 50_000)
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/coupons/validate")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "code": "diskon20",
                "order_amount": 500000.0
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let val_cap: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val_cap["is_valid"], true);
    assert_eq!(val_cap["discount_amount"], 50000.0);
    assert_eq!(val_cap["final_amount"], 450000.0);
}

#[tokio::test]
async fn test_create_and_validate_fixed_coupon() {
    let (app, token, _) = setup_test_app().await;

    // Create fixed coupon
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/coupons")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "code": "HEMAT30K",
                "description": "Potongan langsung Rp30.000",
                "discount_type": "fixed",
                "discount_value": 30000.0,
                "min_order_amount": 60000.0,
                "valid_until": (Utc::now() + Duration::days(14)).to_rfc3339()
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // Validate
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/coupons/validate")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "code": "hemat30k",
                "order_amount": 100000.0
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val["is_valid"], true);
    assert_eq!(val["discount_amount"], 30000.0);
    assert_eq!(val["final_amount"], 70000.0);
}

#[tokio::test]
async fn test_coupon_validations_and_lifecycle() {
    let (app, token, coupon_mod) = setup_test_app().await;

    // 1. Create Coupon
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/coupons")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "code": "SPECIAL10",
                "discount_type": "percentage",
                "discount_value": 10.0,
                "min_order_amount": 100000.0,
                "usage_limit": 1,
                "valid_until": (Utc::now() + Duration::days(1)).to_rfc3339()
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let created: Value = serde_json::from_slice(&body).unwrap();
    let coupon_id = created["id"].as_str().unwrap();

    // 2. Reject if min order not met
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/coupons/validate")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "code": "SPECIAL10",
                "order_amount": 50000.0
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val["is_valid"], false);
    assert!(val["error_message"]
        .as_str()
        .unwrap()
        .contains("Minimum order"));

    // 3. Toggle inactive
    let req = Request::builder()
        .method("PATCH")
        .uri(format!("/api/v1/admin/coupons/{}/status", coupon_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({ "is_active": false }).to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Validate rejected when inactive
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/coupons/validate")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "code": "SPECIAL10",
                "order_amount": 150000.0
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val["is_valid"], false);
    assert!(val["error_message"]
        .as_str()
        .unwrap()
        .contains("tidak aktif"));

    // 4. Toggle back to active
    let req = Request::builder()
        .method("PATCH")
        .uri(format!("/api/v1/admin/coupons/{}/status", coupon_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(json!({ "is_active": true }).to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 5. Simulate recording usage up to limit
    let cid = Uuid::parse_str(coupon_id).unwrap();
    coupon_mod
        .record_usage(cid, None, None, 15000.0)
        .await
        .unwrap();

    // Validate rejected when limit exceeded
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/coupons/validate")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "code": "SPECIAL10",
                "order_amount": 150000.0
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val["is_valid"], false);
    assert!(val["error_message"]
        .as_str()
        .unwrap()
        .contains("Kuota pemakaian"));

    // 6. Delete coupon
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/admin/coupons/{}", coupon_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // Validate now returns not found
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/coupons/validate")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "code": "SPECIAL10",
                "order_amount": 150000.0
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val["is_valid"], false);
    assert!(val["error_message"]
        .as_str()
        .unwrap()
        .contains("tidak ditemukan"));
}

#[tokio::test]
async fn test_coupon_admin_unauthorized() {
    let (app, _, _) = setup_test_app().await;

    // POST without auth header
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/coupons")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "code": "NOAUTH",
                "discount_type": "fixed",
                "discount_value": 1000.0,
                "valid_until": (Utc::now() + Duration::days(1)).to_rfc3339()
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
