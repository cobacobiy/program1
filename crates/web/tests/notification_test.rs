use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tower::ServiceExt;
use uuid::Uuid;

use program1_contracts::{
    AuthContract, BuyerAccountDto, NotificationContract, UserContract,
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
use program1_module_user::UserModule;
use program1_web::{create_app, AppState};

#[allow(dead_code)]
struct TestContext {
    app: axum::Router,
    admin_token: String,
    buyer_token: String,
    buyer_id: Uuid,
    notification_contract: Arc<dyn NotificationContract>,
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

    let order_module = Arc::new(
        OrderModule::new(
            pool.clone(),
            catalog_module.clone(),
            inventory_module.clone(),
        )
        .with_notification_contract(notification_module.clone()),
    );

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

    let buyer_id = Uuid::new_v4();
    let now = Utc::now();
    let buyer_dto = BuyerAccountDto {
        id: buyer_id,
        google_sub: None,
        email: "notif.buyer@test.com".to_string(),
        full_name: "Notif Buyer".to_string(),
        avatar_url: None,
        phone_number: Some("+628123456789".to_string()),
        phone_verified: true,
        is_active: true,
        created_at: now,
        updated_at: now,
    };
    let buyer_token = auth_module.generate_buyer_token(&buyer_dto).unwrap();

    let now_str = now.to_rfc3339();
    sqlx::query(
        "INSERT INTO buyer_accounts (id, email, full_name, phone_number, phone_verified, is_active, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, 1, 1, $5, $6)",
    )
    .bind(buyer_id.to_string())
    .bind(&buyer_dto.email)
    .bind(&buyer_dto.full_name)
    .bind(&buyer_dto.phone_number)
    .bind(&now_str)
    .bind(&now_str)
    .execute(&pool)
    .await
    .unwrap();

    let rate_limiter = Arc::new(program1_web::rate_limit::IpRateLimiter::new());

    let state = AppState {
        store_name: "Notification Test Store".to_string(),
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
        notification_contract: notification_module.clone(),
        rate_limiter,
        started_at: std::time::Instant::now(),
        google_client_id: "test".to_string(),
    };

    let app = create_app(state);

    TestContext {
        app,
        admin_token,
        buyer_token,
        buyer_id,
        notification_contract: notification_module,
        pool,
    }
}

#[tokio::test]
async fn test_buyer_notifications_full_lifecycle() {
    let ctx = setup_test_context().await;

    // 1. Initial count should be 0
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/buyer/notifications/count")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.buyer_token))
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["unread"], 0);

    // 2. Insert 2 buyer notifications directly via contract
    let notif1 = ctx
        .notification_contract
        .create_notification(
            "buyer",
            &ctx.buyer_id.to_string(),
            "Pesanan Dibuat",
            "Pesanan #1001 berhasil dibuat.",
            "order_status",
            Some("1001"),
            Some("order"),
        )
        .await
        .unwrap();

    let _notif2 = ctx
        .notification_contract
        .create_notification(
            "buyer",
            &ctx.buyer_id.to_string(),
            "Promo Kilat",
            "Diskon 50% berlaku hari ini.",
            "promo",
            None,
            None,
        )
        .await
        .unwrap();

    // 3. Check unread count should now be 2
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/buyer/notifications/count")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.buyer_token))
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["unread"], 2);

    // 4. List notifications
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/buyer/notifications?limit=10")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.buyer_token))
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let list: Vec<Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0]["recipient_type"], "buyer");

    // 5. Mark first notification as read
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/buyer/notifications/{}/read", notif1.id))
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.buyer_token))
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Unread count should now be 1
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/buyer/notifications/count")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.buyer_token))
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["unread"], 1);

    // 6. Mark all as read
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/buyer/notifications/read-all")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.buyer_token))
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Unread count should now be 0
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/buyer/notifications/count")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.buyer_token))
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["unread"], 0);
}

#[tokio::test]
async fn test_admin_notifications_full_lifecycle() {
    let ctx = setup_test_context().await;

    // 1. Insert admin notification
    let notif = ctx
        .notification_contract
        .create_notification(
            "seller",
            "admin",
            "Pesanan Baru Masuk",
            "Order ORD-9999 baru saja dibuat.",
            "new_order",
            Some("ORD-9999"),
            Some("order"),
        )
        .await
        .unwrap();

    // 2. Count admin unread notifications
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/admin/notifications/count")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["unread"], 1);

    // 3. List admin notifications
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/admin/notifications?limit=10")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let list: Vec<Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["title"], "Pesanan Baru Masuk");

    // 4. Mark read
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/admin/notifications/{}/read", notif.id))
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 5. Check count after read
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/admin/notifications/count")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["unread"], 0);
}

#[tokio::test]
async fn test_notification_auth_protection() {
    let ctx = setup_test_context().await;

    // Unauthenticated buyer notifications endpoint should be 401
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/buyer/notifications")
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Unauthenticated admin notifications endpoint should be 401
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/admin/notifications")
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Buyer token accessing admin notifications should be 403 Forbidden
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/admin/notifications")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.buyer_token))
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}
