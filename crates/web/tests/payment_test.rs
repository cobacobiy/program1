use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};
use tower::ServiceExt;
use uuid::Uuid;

use program1_contracts::{AuthContract, BuyerAccountDto, UserContract};
use program1_core::init_database;
use program1_module_analytics::AnalyticsModule;
use program1_module_audit::AuditModule;
use program1_module_auth::AuthModule;
use program1_module_buyer::BuyerModule;
use program1_module_catalog::CatalogModule;
use program1_module_channel::ChannelSyncModule;
use program1_module_chat::ChatModule;
use program1_module_inventory::InventoryModule;
use program1_module_order::OrderModule;
use program1_module_payment::PaymentModule;
use program1_module_user::UserModule;
use program1_web::{create_app, AppState};

async fn setup_test_app() -> (axum::Router, String, String, Uuid, Uuid, SqlitePool) {
    let pool = init_database("sqlite::memory:")
        .await
        .expect("Failed to init in-memory sqlite db");

    let user_module = Arc::new(UserModule::new(pool.clone()));
    let auth_module = Arc::new(AuthModule::new(
        "super-secret-jwt-signing-key-for-tests-32chars!".to_string(),
        24,
    ));
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

    let google_verifier = Arc::new(program1_module_buyer::ProductionGoogleVerifier {
        client_id: "test".to_string(),
    });
    let sms_sender = Arc::new(program1_module_buyer::ConsoleOrProviderSmsSender::new(
        "test".to_string(),
        "console".to_string(),
        "".to_string(),
    ));
    let buyer_module = Arc::new(BuyerModule::new(
        pool.clone(),
        auth_module.clone(),
        google_verifier,
        sms_sender,
        audit_module.clone(),
    ));
    let chat_module = Arc::new(ChatModule::new(pool.clone()));
    let payment_module = Arc::new(PaymentModule::new(
        pool.clone(),
        "test-server-key".to_string(),
        "SB-Mid-client-test".to_string(),
        false,
    ));
    let coupon_module = Arc::new(program1_module_coupon::CouponModule::new(pool.clone()));
    let review_module = Arc::new(program1_module_review::ReviewModule::new(pool.clone()));

    let _ = user_module.seed_default_users().await;
    let _ = catalog_module.seed_default_catalog().await;
    let _ = channel_module.seed_default_channels().await;

    let admin_user = user_module.authenticate("admin", "admin123").await.unwrap();
    let admin_token = auth_module.generate_token(&admin_user).unwrap();

    // Create Buyer 1
    let buyer1_id = Uuid::new_v4();
    let buyer1_dto = BuyerAccountDto {
        id: buyer1_id,
        google_sub: None,
        email: "buyer1.payment@test.com".to_string(),
        full_name: "Buyer One".to_string(),
        avatar_url: None,
        phone_number: Some("+628123456789".to_string()),
        phone_verified: true,
        is_active: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let buyer1_token = auth_module.generate_buyer_token(&buyer1_dto).unwrap();

    // Insert buyer1 into buyer_accounts table
    let now_str = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO buyer_accounts (id, email, full_name, phone_number, phone_verified, is_active, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, 1, 1, $5, $6)",
    )
    .bind(buyer1_id.to_string())
    .bind(&buyer1_dto.email)
    .bind(&buyer1_dto.full_name)
    .bind(&buyer1_dto.phone_number)
    .bind(&now_str)
    .bind(&now_str)
    .execute(&pool)
    .await
    .unwrap();

    // Insert shipping address for buyer1
    let addr_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO buyer_addresses (id, buyer_id, recipient_name, phone_number, street_address, subdistrict, city, province, postal_code, is_default, created_at, updated_at) \
         VALUES ($1, $2, 'Buyer One', '+628123456789', 'Jl. Test 1', 'Sub', 'City', 'Prov', '12345', 1, $3, $4)",
    )
    .bind(addr_id.to_string())
    .bind(buyer1_id.to_string())
    .bind(&now_str)
    .bind(&now_str)
    .execute(&pool)
    .await
    .unwrap();

    let rate_limiter = Arc::new(program1_web::rate_limit::IpRateLimiter::new());

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
        rate_limiter,
        started_at: std::time::Instant::now(),
        google_client_id: "test".to_string(),
    };

    (
        create_app(state),
        admin_token,
        buyer1_token,
        buyer1_id,
        addr_id,
        pool,
    )
}

#[tokio::test]
async fn test_payment_config_endpoint() {
    let (app, _, _, _, _, _) = setup_test_app().await;

    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/payments/config")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json_res: Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json_res["client_key"], "SB-Mid-client-test");
    assert_eq!(json_res["is_production"], false);
    assert!(json_res["snap_url"].as_str().unwrap().contains("snap.js"));
}

#[tokio::test]
async fn test_buyer_checkout_and_create_payment_flow() {
    let (app, _, buyer_token, _buyer_id, addr_id, pool) = setup_test_app().await;

    // 1. Get product from catalog
    let catalog_row = sqlx::query("SELECT id FROM catalog_items LIMIT 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    let product_id_str: String = catalog_row.get("id");
    let product_id = Uuid::parse_str(&product_id_str).unwrap();

    // 2. Checkout order as buyer
    let checkout_body = json!({
        "address_id": addr_id,
        "items": [
            { "product_id": product_id, "quantity": 1 }
        ]
    });

    let order_req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/orders")
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&checkout_body).unwrap()))
        .unwrap();

    let order_resp = app.clone().oneshot(order_req).await.unwrap();
    assert_eq!(order_resp.status(), StatusCode::CREATED);
    let order_bytes = to_bytes(order_resp.into_body(), usize::MAX).await.unwrap();
    let order_data: Value = serde_json::from_slice(&order_bytes).unwrap();
    let order_id = order_data["id"].as_str().unwrap().to_string();

    // 3. Create payment for the order
    let pay_body = json!({
        "order_id": order_id
    });

    let pay_req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/payments")
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&pay_body).unwrap()))
        .unwrap();

    let pay_resp = app.clone().oneshot(pay_req).await.unwrap();
    assert_eq!(pay_resp.status(), StatusCode::CREATED);

    let pay_bytes = to_bytes(pay_resp.into_body(), usize::MAX).await.unwrap();
    let payment_data: Value = serde_json::from_slice(&pay_bytes).unwrap();

    assert_eq!(payment_data["order_id"], order_id);
    assert_eq!(payment_data["status"], "pending");
    assert!(payment_data["snap_token"].is_string());
    assert!(payment_data["snap_redirect_url"].is_string());
    let payment_id = payment_data["id"].as_str().unwrap().to_string();

    // 4. Test idempotency: Calling create payment again returns existing payment
    let pay_req2 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/payments")
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&pay_body).unwrap()))
        .unwrap();

    let pay_resp2 = app.clone().oneshot(pay_req2).await.unwrap();
    assert_eq!(pay_resp2.status(), StatusCode::CREATED);
    let pay_bytes2 = to_bytes(pay_resp2.into_body(), usize::MAX).await.unwrap();
    let payment_data2: Value = serde_json::from_slice(&pay_bytes2).unwrap();
    assert_eq!(payment_data2["id"], payment_id);

    // 5. Test Webhook Notification (with signature verification)
    let gross_str = format!("{:.2}", payment_data["amount"].as_f64().unwrap());
    let signature =
        PaymentModule::compute_signature(&order_id, "200", &gross_str, "test-server-key");

    // 5a. Test invalid signature rejection
    let invalid_notif_body = json!({
        "order_id": order_id,
        "status_code": "200",
        "gross_amount": gross_str,
        "signature_key": "invalid_signature_hex",
        "transaction_status": "settlement",
        "payment_type": "qris",
        "transaction_id": "mid-trans-test-123"
    });
    let invalid_notif_req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/payments/notification")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&invalid_notif_body).unwrap()))
        .unwrap();
    let invalid_notif_resp = app.clone().oneshot(invalid_notif_req).await.unwrap();
    assert_eq!(invalid_notif_resp.status(), StatusCode::BAD_REQUEST);

    // 5b. Test valid signature success
    let notification_body = json!({
        "order_id": order_id,
        "status_code": "200",
        "gross_amount": gross_str,
        "signature_key": signature,
        "transaction_status": "settlement",
        "payment_type": "qris",
        "transaction_id": "mid-trans-test-123"
    });

    let notif_req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/payments/notification")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&notification_body).unwrap()))
        .unwrap();

    let notif_resp = app.clone().oneshot(notif_req).await.unwrap();
    assert_eq!(notif_resp.status(), StatusCode::OK);

    // 6. Check payment status via GET /api/v1/payments/order/:order_id
    let get_pay_req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/payments/order/{}", order_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
        .body(Body::empty())
        .unwrap();

    let get_pay_resp = app.clone().oneshot(get_pay_req).await.unwrap();
    assert_eq!(get_pay_resp.status(), StatusCode::OK);
    let get_pay_bytes = to_bytes(get_pay_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let get_pay_data: Value = serde_json::from_slice(&get_pay_bytes).unwrap();

    assert_eq!(get_pay_data["status"], "paid");
    assert_eq!(get_pay_data["payment_method"], "qris");
    assert_eq!(get_pay_data["provider_ref"], "mid-trans-test-123");
    assert!(get_pay_data["paid_at"].is_string());

    // 7. Creating payment for already paid order should return error
    let pay_req3 = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/payments")
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&pay_body).unwrap()))
        .unwrap();

    let pay_resp3 = app.clone().oneshot(pay_req3).await.unwrap();
    assert_eq!(pay_resp3.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_payment_unauthorized_and_ownership_protection() {
    let (app, _, buyer1_token, _, addr_id, pool) = setup_test_app().await;

    // Create Buyer 2
    let auth_module = Arc::new(AuthModule::new(
        "super-secret-jwt-signing-key-for-tests-32chars!".to_string(),
        24,
    ));
    let buyer2_id = Uuid::new_v4();
    let buyer2_dto = BuyerAccountDto {
        id: buyer2_id,
        google_sub: None,
        email: "buyer2.payment@test.com".to_string(),
        full_name: "Buyer Two".to_string(),
        avatar_url: None,
        phone_number: Some("+628987654321".to_string()),
        phone_verified: true,
        is_active: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let buyer2_token = auth_module.generate_buyer_token(&buyer2_dto).unwrap();

    let now_str = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO buyer_accounts (id, email, full_name, phone_number, phone_verified, is_active, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, 1, 1, $5, $6)",
    )
    .bind(buyer2_id.to_string())
    .bind(&buyer2_dto.email)
    .bind(&buyer2_dto.full_name)
    .bind(&buyer2_dto.phone_number)
    .bind(&now_str)
    .bind(&now_str)
    .execute(&pool)
    .await
    .unwrap();

    // 1. Checkout order as Buyer 1
    let catalog_row = sqlx::query("SELECT id FROM catalog_items LIMIT 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    let product_id_str: String = catalog_row.get("id");
    let product_id = Uuid::parse_str(&product_id_str).unwrap();

    let checkout_body = json!({
        "address_id": addr_id,
        "items": [
            { "product_id": product_id, "quantity": 1 }
        ]
    });

    let order_req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/orders")
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer1_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&checkout_body).unwrap()))
        .unwrap();

    let order_resp = app.clone().oneshot(order_req).await.unwrap();
    let order_bytes = to_bytes(order_resp.into_body(), usize::MAX).await.unwrap();
    let order_data: Value = serde_json::from_slice(&order_bytes).unwrap();
    let order_id = order_data["id"].as_str().unwrap().to_string();

    // 2. Unauthenticated request to create payment -> 401
    let unauth_req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/payments")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(&json!({ "order_id": order_id })).unwrap(),
        ))
        .unwrap();

    let unauth_resp = app.clone().oneshot(unauth_req).await.unwrap();
    assert_eq!(unauth_resp.status(), StatusCode::UNAUTHORIZED);

    // 3. Buyer 2 tries to create payment for Buyer 1's order -> 403 Forbidden
    let forbidden_req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/payments")
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer2_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(&json!({ "order_id": order_id })).unwrap(),
        ))
        .unwrap();

    let forbidden_resp = app.clone().oneshot(forbidden_req).await.unwrap();
    assert_eq!(forbidden_resp.status(), StatusCode::FORBIDDEN);

    // 4. Buyer 2 tries to view payment status for Buyer 1's order -> 403 Forbidden
    let forbidden_get = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/payments/order/{}", order_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer2_token))
        .body(Body::empty())
        .unwrap();

    let forbidden_get_resp = app.clone().oneshot(forbidden_get).await.unwrap();
    assert_eq!(forbidden_get_resp.status(), StatusCode::FORBIDDEN);
}
