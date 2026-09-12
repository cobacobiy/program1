use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tower::ServiceExt;
use uuid::Uuid;

use program1_contracts::{
    AuthContract, BuyerAccountDto, UserContract,
};
use program1_core::init_database;
use program1_module_analytics::AnalyticsModule;
use program1_module_audit::AuditModule;
use program1_module_auth::AuthModule;
use program1_module_buyer::BuyerModule;
use program1_module_catalog::CatalogModule;
use program1_module_channel::ChannelSyncModule;
use program1_module_chat::ChatModule;
use program1_module_inventory::InventoryModule;
use program1_module_notification::NotificationModule;
use program1_module_order::OrderModule;
use program1_module_payment::PaymentModule;
use program1_module_return::ReturnModule;
use program1_module_user::UserModule;
use program1_web::{create_app, AppState};

#[allow(dead_code)]
struct TestContext {
    app: axum::Router,
    admin_token: String,
    buyer1_token: String,
    buyer1_id: Uuid,
    buyer2_token: String,
    buyer2_id: Uuid,
    pool: SqlitePool,
}

async fn setup_test_context() -> TestContext {
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
    let notification_module = Arc::new(NotificationModule::new(pool.clone()));
    let audit_module = Arc::new(AuditModule::new(pool.clone()));

    let order_module = Arc::new(
        OrderModule::new(
            pool.clone(),
            catalog_module.clone(),
            inventory_module.clone(),
        )
        .with_notification_contract(notification_module.clone()),
    );

    let return_module = Arc::new(
        ReturnModule::new(pool.clone())
            .with_inventory_contract(inventory_module.clone())
            .with_notification_contract(notification_module.clone())
            .with_audit_contract(audit_module.clone()),
    );

    let analytics_module = Arc::new(AnalyticsModule::new(
        catalog_module.clone(),
        order_module.clone(),
    ));

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
    let shipping_module = Arc::new(program1_module_shipping::ShippingModule::new(
        "".to_string(),
        "starter".to_string(),
        "152".to_string(),
    ));

    let _ = user_module.seed_default_users().await;
    let _ = catalog_module.seed_default_catalog().await;
    let _ = channel_module.seed_default_channels().await;

    let admin_user = user_module.authenticate("admin", "admin123").await.unwrap();
    let admin_token = auth_module.generate_token(&admin_user).unwrap();

    let now = Utc::now();
    let buyer1_id = Uuid::new_v4();
    let buyer1_dto = BuyerAccountDto {
        id: buyer1_id,
        google_sub: None,
        email: "buyer1@test.com".to_string(),
        full_name: "Buyer One".to_string(),
        avatar_url: None,
        phone_number: Some("+628111111111".to_string()),
        phone_verified: true,
        is_active: true,
        created_at: now,
        updated_at: now,
    };
    let buyer1_token = auth_module.generate_buyer_token(&buyer1_dto).unwrap();

    let buyer2_id = Uuid::new_v4();
    let buyer2_dto = BuyerAccountDto {
        id: buyer2_id,
        google_sub: None,
        email: "buyer2@test.com".to_string(),
        full_name: "Buyer Two".to_string(),
        avatar_url: None,
        phone_number: Some("+628222222222".to_string()),
        phone_verified: true,
        is_active: true,
        created_at: now,
        updated_at: now,
    };
    let buyer2_token = auth_module.generate_buyer_token(&buyer2_dto).unwrap();

    let now_str = now.to_rfc3339();
    sqlx::query(
        "INSERT INTO buyer_accounts (id, email, full_name, phone_number, phone_verified, is_active, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, 1, 1, $5, $6), ($7, $8, $9, $10, 1, 1, $5, $6)",
    )
    .bind(buyer1_id.to_string())
    .bind(&buyer1_dto.email)
    .bind(&buyer1_dto.full_name)
    .bind(&buyer1_dto.phone_number)
    .bind(&now_str)
    .bind(&now_str)
    .bind(buyer2_id.to_string())
    .bind(&buyer2_dto.email)
    .bind(&buyer2_dto.full_name)
    .bind(&buyer2_dto.phone_number)
    .execute(&pool)
    .await
    .unwrap();

    let rate_limiter = Arc::new(program1_web::rate_limit::IpRateLimiter::new());

    let state = AppState {
        store_name: "Return Test Store".to_string(),
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
        shipping_contract: shipping_module,
        notification_contract: notification_module,
        return_contract: return_module,
        backup_contract: Arc::new(program1_core::backup::BackupService::new(pool.clone(), "./target/test_backups")),
        rate_limiter,
        started_at: std::time::Instant::now(),
        google_client_id: "test".to_string(),
    };

    let app = create_app(state);

    TestContext {
        app,
        admin_token,
        buyer1_token,
        buyer1_id,
        buyer2_token,
        buyer2_id,
        pool,
    }
}

async fn create_test_order(pool: &SqlitePool, buyer_id: &Uuid, status: &str) -> (Uuid, Uuid) {
    let order_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO catalog_items (id, sku, name, price, stock, created_at) \
         VALUES ($1, $2, 'Test Keyboard', 150000.0, 10, $3)",
    )
    .bind(product_id.to_string())
    .bind(format!("SKU-{}", Uuid::new_v4().to_string()[..8].to_string()))
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO orders (id, buyer_id, customer_name, customer_email, total_amount, status, created_at) \
         VALUES ($1, $2, 'Buyer Test', 'buyer@test.com', 150000.0, $3, $4)",
    )
    .bind(order_id.to_string())
    .bind(buyer_id.to_string())
    .bind(status)
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO order_items (id, order_id, product_id, product_name, quantity, unit_price, total_price) \
         VALUES ($1, $2, $3, 'Test Keyboard', 2, 75000.0, 150000.0)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(order_id.to_string())
    .bind(product_id.to_string())
    .execute(pool)
    .await
    .unwrap();

    (order_id, product_id)
}

#[tokio::test]
async fn test_create_return_for_delivered_order() {
    let ctx = setup_test_context().await;
    let (order_id, _) = create_test_order(&ctx.pool, &ctx.buyer1_id, "delivered").await;

    let payload = json!({
        "order_id": order_id.to_string(),
        "reason": "defective",
        "description": "Keys are sticky and not working properly",
        "evidence_urls": ["https://example.com/photo1.jpg"]
    });

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/buyer/returns")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.buyer1_token))
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let response = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["id"].is_string());
    assert_eq!(json["order_id"], order_id.to_string());
    assert_eq!(json["reason"], "defective");
    assert_eq!(json["status"], "pending");
}

#[tokio::test]
async fn test_cannot_return_non_delivered_order() {
    let ctx = setup_test_context().await;
    let (order_id, _) = create_test_order(&ctx.pool, &ctx.buyer1_id, "shipped").await;

    let payload = json!({
        "order_id": order_id.to_string(),
        "reason": "defective",
        "description": "Item not received yet but trying to return",
        "evidence_urls": []
    });

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/buyer/returns")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.buyer1_token))
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let response = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_cannot_return_other_buyer_order() {
    let ctx = setup_test_context().await;
    let (order_id, _) = create_test_order(&ctx.pool, &ctx.buyer1_id, "delivered").await;

    let payload = json!({
        "order_id": order_id.to_string(),
        "reason": "wrong_item",
        "description": "Buyer 2 attempting to return Buyer 1 order",
        "evidence_urls": []
    });

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/buyer/returns")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.buyer2_token))
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let response = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_admin_approve_and_lifecycle() {
    let ctx = setup_test_context().await;
    let (order_id, product_id) = create_test_order(&ctx.pool, &ctx.buyer1_id, "completed").await;

    // 1. Buyer submits return request
    let payload = json!({
        "order_id": order_id.to_string(),
        "reason": "defective",
        "description": "Defective product switch",
        "evidence_urls": []
    });

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/buyer/returns")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.buyer1_token))
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let response = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let res_json: Value = serde_json::from_slice(&body).unwrap();
    let return_id = res_json["id"].as_str().unwrap();

    // 2. Admin approves return request
    let process_payload = json!({
        "status": "approved",
        "admin_notes": "Approved for return & refund",
        "refund_amount_cents": 1500000
    });

    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/admin/returns/{}/process", return_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
        .body(Body::from(serde_json::to_vec(&process_payload).unwrap()))
        .unwrap();

    let response = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 3. Buyer marks status to return_shipped
    let update_payload = json!({
        "status": "return_shipped",
        "admin_notes": null
    });

    let req = Request::builder()
        .method(Method::PUT)
        .uri(format!("/api/v1/buyer/returns/{}/status", return_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.buyer1_token))
        .body(Body::from(serde_json::to_vec(&update_payload).unwrap()))
        .unwrap();

    let response = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 4. Admin updates status to received (triggers inventory restock)
    let update_payload_admin = json!({
        "status": "received",
        "admin_notes": "Returned package received in warehouse"
    });

    let req = Request::builder()
        .method(Method::PUT)
        .uri(format!("/api/v1/admin/returns/{}/status", return_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
        .body(Body::from(serde_json::to_vec(&update_payload_admin).unwrap()))
        .unwrap();

    let response = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Check stock was restocked (+2 units)
    let row: (i32,) = sqlx::query_as("SELECT stock FROM catalog_items WHERE id = $1")
        .bind(product_id.to_string())
        .fetch_one(&ctx.pool)
        .await
        .unwrap();
    assert_eq!(row.0, 12); // initial 10 + 2 restocked = 12

    // 5. Admin updates status to refunded
    let update_payload_refunded = json!({
        "status": "refunded",
        "admin_notes": "Funds returned to buyer account"
    });

    let req = Request::builder()
        .method(Method::PUT)
        .uri(format!("/api/v1/admin/returns/{}/status", return_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
        .body(Body::from(serde_json::to_vec(&update_payload_refunded).unwrap()))
        .unwrap();

    let response = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_reject_return_requires_notes() {
    let ctx = setup_test_context().await;
    let (order_id, _) = create_test_order(&ctx.pool, &ctx.buyer1_id, "delivered").await;

    // Buyer creates return
    let payload = json!({
        "order_id": order_id.to_string(),
        "reason": "other",
        "description": "Changed my mind after 30 days",
        "evidence_urls": []
    });

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/buyer/returns")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.buyer1_token))
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let response = ctx.app.clone().oneshot(req).await.unwrap();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let res_json: Value = serde_json::from_slice(&body).unwrap();
    let return_id = res_json["id"].as_str().unwrap();

    // Admin attempts to reject without admin_notes (should fail validation)
    let reject_payload_empty = json!({
        "status": "rejected",
        "admin_notes": "",
        "refund_amount_cents": null
    });

    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/admin/returns/{}/process", return_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
        .body(Body::from(serde_json::to_vec(&reject_payload_empty).unwrap()))
        .unwrap();

    let response = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Admin rejects with valid admin_notes
    let reject_payload_valid = json!({
        "status": "rejected",
        "admin_notes": "Return request rejected as it exceeds the 14-day window",
        "refund_amount_cents": null
    });

    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/admin/returns/{}/process", return_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
        .body(Body::from(serde_json::to_vec(&reject_payload_valid).unwrap()))
        .unwrap();

    let response = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
