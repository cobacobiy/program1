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

#[allow(dead_code)]
struct TestContext {
    app: axum::Router,
    admin_token: String,
    buyer1_token: String,
    buyer1_id: Uuid,
    buyer1_addr_id: Uuid,
    buyer2_token: String,
    buyer2_id: Uuid,
    buyer2_addr_id: Uuid,
    product_id: Uuid,
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

    let now_str = Utc::now().to_rfc3339();

    // Buyer 1
    let buyer1_id = Uuid::new_v4();
    let buyer1_dto = BuyerAccountDto {
        id: buyer1_id,
        google_sub: None,
        email: "buyer1.status@test.com".to_string(),
        full_name: "Buyer One".to_string(),
        avatar_url: None,
        phone_number: Some("+628123456789".to_string()),
        phone_verified: true,
        is_active: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let buyer1_token = auth_module.generate_buyer_token(&buyer1_dto).unwrap();

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

    let buyer1_addr_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO buyer_addresses (id, buyer_id, recipient_name, phone_number, street_address, subdistrict, city, province, postal_code, is_default, created_at, updated_at) \
         VALUES ($1, $2, 'Buyer One', '+628123456789', 'Jl. Merdeka No. 45', 'Gambir', 'Jakarta Pusat', 'DKI Jakarta', '10110', 1, $3, $4)",
    )
    .bind(buyer1_addr_id.to_string())
    .bind(buyer1_id.to_string())
    .bind(&now_str)
    .bind(&now_str)
    .execute(&pool)
    .await
    .unwrap();

    // Buyer 2
    let buyer2_id = Uuid::new_v4();
    let buyer2_dto = BuyerAccountDto {
        id: buyer2_id,
        google_sub: None,
        email: "buyer2.status@test.com".to_string(),
        full_name: "Buyer Two".to_string(),
        avatar_url: None,
        phone_number: Some("+628987654321".to_string()),
        phone_verified: true,
        is_active: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let buyer2_token = auth_module.generate_buyer_token(&buyer2_dto).unwrap();

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

    let buyer2_addr_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO buyer_addresses (id, buyer_id, recipient_name, phone_number, street_address, subdistrict, city, province, postal_code, is_default, created_at, updated_at) \
         VALUES ($1, $2, 'Buyer Two', '+628987654321', 'Jl. Sudirman No. 10', 'Setiabudi', 'Jakarta Selatan', 'DKI Jakarta', '12920', 1, $3, $4)",
    )
    .bind(buyer2_addr_id.to_string())
    .bind(buyer2_id.to_string())
    .bind(&now_str)
    .bind(&now_str)
    .execute(&pool)
    .await
    .unwrap();

    // Query a catalog product
    let cat_row = sqlx::query("SELECT id FROM catalog_items LIMIT 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    let product_id_str: String = cat_row.get("id");
    let product_id = Uuid::parse_str(&product_id_str).unwrap();

    let rate_limiter = Arc::new(program1_web::rate_limit::IpRateLimiter::new());

    let state = AppState {
        store_name: "Status Test Store".to_string(),
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
        buyer1_addr_id,
        buyer2_token,
        buyer2_id,
        buyer2_addr_id,
        product_id,
        pool,
    }
}

async fn create_order(
    app: &axum::Router,
    token: &str,
    address_id: Uuid,
    product_id: Uuid,
) -> String {
    let order_payload = json!({
        "address_id": address_id,
        "items": [
            {
                "product_id": product_id,
                "quantity": 1
            }
        ]
    });

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/orders")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(serde_json::to_vec(&order_payload).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    body["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn test_full_order_status_lifecycle() {
    let ctx = setup_test_context().await;
    let order_id = create_order(
        &ctx.app,
        &ctx.buyer1_token,
        ctx.buyer1_addr_id,
        ctx.product_id,
    )
    .await;

    // 1. Initial status should be "pending"
    let row = sqlx::query(
        "SELECT status, tracking_number, shipped_at, delivered_at FROM orders WHERE id = $1",
    )
    .bind(&order_id)
    .fetch_one(&ctx.pool)
    .await
    .unwrap();
    let status: String = row.get("status");
    assert_eq!(status, "pending");

    // 2. Transition pending -> paid (Admin)
    let patch_req = Request::builder()
        .method(Method::PATCH)
        .uri(format!("/api/v1/orders/{}/status", order_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
        .body(Body::from(
            serde_json::to_vec(&json!({ "status": "paid" })).unwrap(),
        ))
        .unwrap();

    let res = ctx.app.clone().oneshot(patch_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["status"], "paid");

    // 3. Transition paid -> processing (Admin)
    let patch_req = Request::builder()
        .method(Method::PATCH)
        .uri(format!("/api/v1/orders/{}/status", order_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
        .body(Body::from(
            serde_json::to_vec(&json!({ "status": "processing" })).unwrap(),
        ))
        .unwrap();

    let res = ctx.app.clone().oneshot(patch_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["status"], "processing");

    // 4. Transition processing -> shipped with tracking number (Admin)
    let patch_req = Request::builder()
        .method(Method::PATCH)
        .uri(format!("/api/v1/orders/{}/status", order_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
        .body(Body::from(
            serde_json::to_vec(&json!({
                "status": "shipped",
                "tracking_number": "JNE-99887766"
            }))
            .unwrap(),
        ))
        .unwrap();

    let res = ctx.app.clone().oneshot(patch_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["status"], "shipped");
    assert_eq!(body["tracking_number"], "JNE-99887766");
    assert!(body["shipped_at"].is_string());

    // 5. Transition shipped -> delivered via Buyer Confirm Delivery
    let confirm_req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/orders/{}/confirm-delivery", order_id))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", ctx.buyer1_token),
        )
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.clone().oneshot(confirm_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["status"], "delivered");
    assert!(body["delivered_at"].is_string());

    // 6. Transition delivered -> completed (Admin)
    let patch_req = Request::builder()
        .method(Method::PATCH)
        .uri(format!("/api/v1/orders/{}/status", order_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
        .body(Body::from(
            serde_json::to_vec(&json!({ "status": "completed" })).unwrap(),
        ))
        .unwrap();

    let res = ctx.app.clone().oneshot(patch_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["status"], "completed");
}

#[tokio::test]
async fn test_invalid_order_status_transitions() {
    let ctx = setup_test_context().await;
    let order_id = create_order(
        &ctx.app,
        &ctx.buyer1_token,
        ctx.buyer1_addr_id,
        ctx.product_id,
    )
    .await;

    // Direct transition pending -> shipped is illegal
    let patch_req = Request::builder()
        .method(Method::PATCH)
        .uri(format!("/api/v1/orders/{}/status", order_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
        .body(Body::from(
            serde_json::to_vec(&json!({ "status": "shipped" })).unwrap(),
        ))
        .unwrap();

    let res = ctx.app.clone().oneshot(patch_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Direct transition pending -> delivered is illegal
    let patch_req = Request::builder()
        .method(Method::PATCH)
        .uri(format!("/api/v1/orders/{}/status", order_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
        .body(Body::from(
            serde_json::to_vec(&json!({ "status": "delivered" })).unwrap(),
        ))
        .unwrap();

    let res = ctx.app.clone().oneshot(patch_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_buyer_cancel_pending_order() {
    let ctx = setup_test_context().await;
    let order_id = create_order(
        &ctx.app,
        &ctx.buyer1_token,
        ctx.buyer1_addr_id,
        ctx.product_id,
    )
    .await;

    // Buyer cancels pending order
    let cancel_req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/orders/{}/cancel", order_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", ctx.buyer1_token),
        )
        .body(Body::from(
            serde_json::to_vec(&json!({
                "reason": "Salah beli varian"
            }))
            .unwrap(),
        ))
        .unwrap();

    let res = ctx.app.clone().oneshot(cancel_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["status"], "cancelled");
    assert_eq!(body["cancel_reason"], "Salah beli varian");
    assert_eq!(body["cancelled_by"], format!("buyer:{}", ctx.buyer1_id));
    assert!(body["cancelled_at"].is_string());

    // Trying to cancel again should fail (cancelled order has no transitions)
    let cancel_req_again = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/orders/{}/cancel", order_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", ctx.buyer1_token),
        )
        .body(Body::from(
            serde_json::to_vec(&json!({ "reason": "Lagi" })).unwrap(),
        ))
        .unwrap();

    let res = ctx.app.clone().oneshot(cancel_req_again).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_buyer_cannot_cancel_shipped_order() {
    let ctx = setup_test_context().await;
    let order_id = create_order(
        &ctx.app,
        &ctx.buyer1_token,
        ctx.buyer1_addr_id,
        ctx.product_id,
    )
    .await;

    // Advance order to shipped: pending -> paid -> processing -> shipped
    for next_status in ["paid", "processing", "shipped"] {
        let req = Request::builder()
            .method(Method::PATCH)
            .uri(format!("/api/v1/orders/{}/status", order_id))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
            .body(Body::from(
                serde_json::to_vec(&json!({ "status": next_status })).unwrap(),
            ))
            .unwrap();
        let res = ctx.app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    // Buyer tries to cancel shipped order -> rejected
    let cancel_req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/orders/{}/cancel", order_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", ctx.buyer1_token),
        )
        .body(Body::from(
            serde_json::to_vec(&json!({ "reason": "Ingin batal" })).unwrap(),
        ))
        .unwrap();

    let res = ctx.app.clone().oneshot(cancel_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_buyer_cannot_cancel_or_confirm_other_buyer_order() {
    let ctx = setup_test_context().await;
    // Order belongs to Buyer 1
    let order_id = create_order(
        &ctx.app,
        &ctx.buyer1_token,
        ctx.buyer1_addr_id,
        ctx.product_id,
    )
    .await;

    // Buyer 2 attempts to cancel Buyer 1's order -> 403 Forbidden
    let cancel_req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/orders/{}/cancel", order_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", ctx.buyer2_token),
        )
        .body(Body::from(
            serde_json::to_vec(&json!({ "reason": "Hacking" })).unwrap(),
        ))
        .unwrap();

    let res = ctx.app.clone().oneshot(cancel_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // Advance to shipped by Admin
    for next_status in ["paid", "processing", "shipped"] {
        let req = Request::builder()
            .method(Method::PATCH)
            .uri(format!("/api/v1/orders/{}/status", order_id))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
            .body(Body::from(
                serde_json::to_vec(&json!({ "status": next_status })).unwrap(),
            ))
            .unwrap();
        let res = ctx.app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    // Buyer 2 attempts to confirm delivery of Buyer 1's order -> 403 Forbidden
    let confirm_req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/orders/{}/confirm-delivery", order_id))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", ctx.buyer2_token),
        )
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.clone().oneshot(confirm_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_buyer_confirm_delivery_validation() {
    let ctx = setup_test_context().await;
    let order_id = create_order(
        &ctx.app,
        &ctx.buyer1_token,
        ctx.buyer1_addr_id,
        ctx.product_id,
    )
    .await;

    // Cannot confirm delivery while still "pending"
    let confirm_req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/orders/{}/confirm-delivery", order_id))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", ctx.buyer1_token),
        )
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.clone().oneshot(confirm_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_list_buyer_orders_isolation() {
    let ctx = setup_test_context().await;

    // Buyer 1 creates 2 orders
    let o1 = create_order(
        &ctx.app,
        &ctx.buyer1_token,
        ctx.buyer1_addr_id,
        ctx.product_id,
    )
    .await;
    let o2 = create_order(
        &ctx.app,
        &ctx.buyer1_token,
        ctx.buyer1_addr_id,
        ctx.product_id,
    )
    .await;

    // Buyer 2 creates 1 order
    let o3 = create_order(
        &ctx.app,
        &ctx.buyer2_token,
        ctx.buyer2_addr_id,
        ctx.product_id,
    )
    .await;

    // Unauthenticated GET /api/v1/buyer/orders -> 401
    let unauth_req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/buyer/orders")
        .body(Body::empty())
        .unwrap();
    let res = ctx.app.clone().oneshot(unauth_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Buyer 1 queries orders
    let req1 = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/buyer/orders")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", ctx.buyer1_token),
        )
        .body(Body::empty())
        .unwrap();
    let res1 = ctx.app.clone().oneshot(req1).await.unwrap();
    assert_eq!(res1.status(), StatusCode::OK);
    let bytes1 = to_bytes(res1.into_body(), usize::MAX).await.unwrap();
    let list1: Vec<Value> = serde_json::from_slice(&bytes1).unwrap();
    assert_eq!(list1.len(), 2);
    let ids1: Vec<&str> = list1.iter().map(|o| o["id"].as_str().unwrap()).collect();
    assert!(ids1.contains(&o1.as_str()));
    assert!(ids1.contains(&o2.as_str()));
    assert!(!ids1.contains(&o3.as_str()));

    // Buyer 2 queries orders
    let req2 = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/buyer/orders")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", ctx.buyer2_token),
        )
        .body(Body::empty())
        .unwrap();
    let res2 = ctx.app.clone().oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::OK);
    let bytes2 = to_bytes(res2.into_body(), usize::MAX).await.unwrap();
    let list2: Vec<Value> = serde_json::from_slice(&bytes2).unwrap();
    assert_eq!(list2.len(), 1);
    assert_eq!(list2[0]["id"].as_str().unwrap(), o3);
}

#[tokio::test]
async fn test_audit_logging_on_status_change() {
    let ctx = setup_test_context().await;
    let order_id = create_order(
        &ctx.app,
        &ctx.buyer1_token,
        ctx.buyer1_addr_id,
        ctx.product_id,
    )
    .await;

    // 1. Status update by admin
    let patch_req = Request::builder()
        .method(Method::PATCH)
        .uri(format!("/api/v1/orders/{}/status", order_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
        .body(Body::from(
            serde_json::to_vec(&json!({ "status": "paid" })).unwrap(),
        ))
        .unwrap();

    let res = ctx.app.clone().oneshot(patch_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Verify audit log has ORDER_STATUS_UPDATED
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs WHERE action = 'ORDER_STATUS_UPDATED' AND resource_id = $1"
    )
    .bind(&order_id)
    .fetch_one(&ctx.pool)
    .await
    .unwrap();
    assert_eq!(audit_count, 1);
}

#[tokio::test]
async fn test_get_buyer_order_detail_and_isolation() {
    let ctx = setup_test_context().await;
    let o1 = create_order(
        &ctx.app,
        &ctx.buyer1_token,
        ctx.buyer1_addr_id,
        ctx.product_id,
    )
    .await;
    let o2 = create_order(
        &ctx.app,
        &ctx.buyer2_token,
        ctx.buyer2_addr_id,
        ctx.product_id,
    )
    .await;

    // 1. Unauthenticated -> 401
    let unauth_req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/buyer/orders/{}", o1))
        .body(Body::empty())
        .unwrap();
    let res = ctx.app.clone().oneshot(unauth_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 2. Buyer 1 gets own order -> 200 OK
    let req1 = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/buyer/orders/{}", o1))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", ctx.buyer1_token),
        )
        .body(Body::empty())
        .unwrap();
    let res1 = ctx.app.clone().oneshot(req1).await.unwrap();
    assert_eq!(res1.status(), StatusCode::OK);
    let bytes1 = to_bytes(res1.into_body(), usize::MAX).await.unwrap();
    let order_data: Value = serde_json::from_slice(&bytes1).unwrap();
    assert_eq!(order_data["id"].as_str().unwrap(), o1);
    assert_eq!(
        order_data["buyer_id"].as_str().unwrap(),
        ctx.buyer1_id.to_string()
    );
    assert_eq!(order_data["status"].as_str().unwrap(), "pending");

    // 3. Buyer 1 attempts to access Buyer 2's order -> 404 NOT_FOUND (ownership check)
    let forbidden_req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/buyer/orders/{}", o2))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", ctx.buyer1_token),
        )
        .body(Body::empty())
        .unwrap();
    let forbidden_res = ctx.app.clone().oneshot(forbidden_req).await.unwrap();
    assert_eq!(forbidden_res.status(), StatusCode::NOT_FOUND);

    // 4. Non-existent order -> 404 NOT_FOUND
    let not_found_req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/buyer/orders/{}", Uuid::new_v4()))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", ctx.buyer1_token),
        )
        .body(Body::empty())
        .unwrap();
    let not_found_res = ctx.app.clone().oneshot(not_found_req).await.unwrap();
    assert_eq!(not_found_res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_buyer_profile_endpoint() {
    let ctx = setup_test_context().await;

    // 1. Unauthenticated PUT -> 401
    let unauth_req = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/buyer/profile")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(&json!({
                "full_name": "Budi Santoso Baru"
            }))
            .unwrap(),
        ))
        .unwrap();
    let unauth_res = ctx.app.clone().oneshot(unauth_req).await.unwrap();
    assert_eq!(unauth_res.status(), StatusCode::UNAUTHORIZED);

    // 2. Buyer updates full_name and avatar_url -> 200 OK
    let update_req = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/buyer/profile")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", ctx.buyer1_token),
        )
        .body(Body::from(
            serde_json::to_vec(&json!({
                "full_name": "Budi Santoso Baru",
                "avatar_url": "https://example.com/avatar.jpg"
            }))
            .unwrap(),
        ))
        .unwrap();
    let update_res = ctx.app.clone().oneshot(update_req).await.unwrap();
    assert_eq!(update_res.status(), StatusCode::OK);
    let bytes = to_bytes(update_res.into_body(), usize::MAX).await.unwrap();
    let profile: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(profile["full_name"].as_str().unwrap(), "Budi Santoso Baru");
    assert_eq!(
        profile["avatar_url"].as_str().unwrap(),
        "https://example.com/avatar.jpg"
    );

    // 3. GET /api/v1/buyer/profile reflects updated profile
    let get_req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/buyer/profile")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", ctx.buyer1_token),
        )
        .body(Body::empty())
        .unwrap();
    let get_res = ctx.app.clone().oneshot(get_req).await.unwrap();
    assert_eq!(get_res.status(), StatusCode::OK);
    let get_bytes = to_bytes(get_res.into_body(), usize::MAX).await.unwrap();
    let get_profile: Value = serde_json::from_slice(&get_bytes).unwrap();
    assert_eq!(
        get_profile["full_name"].as_str().unwrap(),
        "Budi Santoso Baru"
    );

    // 4. Invalid validation (name too short) -> 422 Unprocessable Entity
    let invalid_req = Request::builder()
        .method(Method::PUT)
        .uri("/api/v1/buyer/profile")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", ctx.buyer1_token),
        )
        .body(Body::from(
            serde_json::to_vec(&json!({
                "full_name": "x"
            }))
            .unwrap(),
        ))
        .unwrap();
    let invalid_res = ctx.app.clone().oneshot(invalid_req).await.unwrap();
    assert_eq!(invalid_res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
