use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use program1_contracts::*;
use program1_core::database::init_database;
use program1_module_analytics::AnalyticsModule;
use program1_module_audit::AuditModule;
use program1_module_auth::AuthModule;
use program1_module_buyer::BuyerModule;
use program1_module_catalog::CatalogModule;
use program1_module_channel::ChannelSyncModule;
use program1_module_chat::ChatModule;
use program1_module_coupon::CouponModule;
use program1_module_flash_sale::FlashSaleModule;
use program1_module_inventory::InventoryModule;
use program1_module_order::OrderModule;
use program1_module_payment::PaymentModule;
use program1_module_review::ReviewModule;
use program1_module_shipping::ShippingModule;
use program1_module_user::UserModule;
use program1_web::{create_app, state::AppState};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

async fn setup_test_app() -> (
    axum::Router,
    Arc<dyn FlashSaleContract>,
    Arc<dyn CatalogContract>,
    String, // admin token
    String, // buyer token
) {
    let secret = "test-jwt-secret-key-minimum-32-characters-length!".to_string();
    let pool = init_database("sqlite::memory:").await.unwrap();

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
    let audit_module = Arc::new(AuditModule::new(pool.clone()));
    let google_verifier = Arc::new(program1_module_buyer::ProductionGoogleVerifier {
        client_id: "test".to_string(),
    });
    let sms_sender = Arc::new(program1_module_buyer::ConsoleOrProviderSmsSender::default());
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
        "mock_key".to_string(),
        "mock_client".to_string(),
        false,
    ));
    let coupon_module = Arc::new(CouponModule::new(pool.clone()));
    let review_module = Arc::new(ReviewModule::new(pool.clone()));
    let shipping_module = Arc::new(ShippingModule::new(
        "mock_rajaongkir".to_string(),
        "starter".to_string(),
        "152".to_string(),
    ));
    let flash_sale_module = Arc::new(FlashSaleModule::new(pool.clone()));

    user_module.seed_default_users().await.unwrap();
    catalog_module.seed_default_catalog().await.unwrap();

    let state = AppState {
        store_name: "AURA Storefront".to_string(),
        store_currency: "IDR".to_string(),
        store_whatsapp_number: "+6281234567890".to_string(),
        user_contract: user_module.clone(),
        auth_contract: auth_module.clone(),
        catalog_contract: catalog_module.clone(),
        inventory_contract: inventory_module,
        channel_contract: channel_module,
        order_contract: order_module,
        analytics_contract: analytics_module,
        audit_contract: audit_module,
        buyer_contract: buyer_module.clone(),
        chat_contract: chat_module,
        payment_contract: payment_module,
        coupon_contract: coupon_module,
        review_contract: review_module,
        shipping_contract: shipping_module,
        notification_contract: Arc::new(program1_module_notification::NotificationModule::new(
            pool.clone(),
        )),
        return_contract: Arc::new(program1_module_return::ReturnModule::new(pool.clone())),
        backup_contract: Arc::new(program1_core::backup::BackupService::new(
            pool.clone(),
            "./target/test_backups",
        )),
        flash_sale_contract: flash_sale_module.clone(),
        supplier_contract: Arc::new(program1_module_supplier::SupplierModule::new(pool.clone())),
        rate_limiter: Arc::new(program1_web::rate_limit::IpRateLimiter::new()),
        started_at: std::time::Instant::now(),
        google_client_id: "test".to_string(),
    };

    let app = create_app(state);

    let now_ts = Utc::now().timestamp();
    let admin_claims = JwtClaims {
        sub: Uuid::new_v4(),
        username: "admin".to_string(),
        role: "admin".to_string(),
        accessible_menus: vec!["all".to_string()],
        exp: now_ts + 3600,
        iat: now_ts,
        user_type: "seller_staff".to_string(),
        permissions: vec!["*".to_string()],
    };
    let admin_token = encode(
        &Header::default(),
        &admin_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap();

    let buyer_claims = JwtClaims {
        sub: Uuid::new_v4(),
        username: "buyer@example.com".to_string(),
        role: "buyer".to_string(),
        accessible_menus: vec![],
        exp: now_ts + 3600,
        iat: now_ts,
        user_type: "buyer".to_string(),
        permissions: vec![],
    };
    let buyer_token = encode(
        &Header::default(),
        &buyer_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap();

    (
        app,
        flash_sale_module,
        catalog_module,
        admin_token,
        buyer_token,
    )
}

#[tokio::test]
async fn test_flash_sale_active_session_and_countdown() {
    let (app, flash_sale, catalog, _admin_token, _) = setup_test_app().await;

    // Create an active session spanning from 1 hour ago to 2 hours in the future
    let now = Utc::now();
    let start = (now - Duration::hours(1)).to_rfc3339();
    let end = (now + Duration::hours(2)).to_rfc3339();

    let session = flash_sale
        .create_session(CreateFlashSaleSessionRequest {
            title: "Super Midnight Flash Sale 9.9".to_string(),
            start_time: start,
            end_time: end,
            banner_url: Some("https://example.com/banner.jpg".to_string()),
        })
        .await
        .unwrap();

    let items = catalog.list_items().await.unwrap();
    let product_id = items[0].id.to_string();

    flash_sale
        .add_item_to_session(
            &session.id,
            AddFlashSaleItemRequest {
                product_id: product_id.clone(),
                flash_price_cents: 99000,
                allocated_quantity: 20,
            },
        )
        .await
        .unwrap();

    // Query active session via public endpoint
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/flash-sales/active")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let active_session: Option<FlashSaleSessionDto> = serde_json::from_slice(&body).unwrap();

    assert!(active_session.is_some());
    let active = active_session.unwrap();
    assert_eq!(active.title, "Super Midnight Flash Sale 9.9");
    assert!(active.remaining_seconds.is_some());
    assert!(active.remaining_seconds.unwrap() > 0);
    assert_eq!(active.items.len(), 1);
    assert_eq!(active.items[0].flash_price_cents, 99000);
}

#[tokio::test]
async fn test_flash_sale_admin_protection_and_crud() {
    let (app, _flash_sale, _catalog, admin_token, buyer_token) = setup_test_app().await;

    let now = Utc::now();
    let payload = json!({
        "title": "Admin Created Flash Sale",
        "start_time": (now + Duration::hours(1)).to_rfc3339(),
        "end_time": (now + Duration::hours(3)).to_rfc3339(),
        "banner_url": "https://example.com/banner.png"
    });

    // 1. Unauthorized request (no token) -> 401
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/flash-sales")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 2. Forbidden request (Buyer token) -> 403
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/flash-sales")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
        .body(Body::from(payload.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // 3. Authorized request (Admin token) -> 200 OK
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/flash-sales")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .body(Body::from(payload.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_flash_sale_toggle_and_expiration() {
    let (app, flash_sale, _catalog, admin_token, _) = setup_test_app().await;

    let now = Utc::now();
    let session = flash_sale
        .create_session(CreateFlashSaleSessionRequest {
            title: "Toggle Test Session".to_string(),
            start_time: (now - Duration::minutes(30)).to_rfc3339(),
            end_time: (now + Duration::minutes(30)).to_rfc3339(),
            banner_url: None,
        })
        .await
        .unwrap();

    // Verify it is active initially
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/flash-sales/active")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let body = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let active: Option<FlashSaleSessionDto> = serde_json::from_slice(&body).unwrap();
    assert!(active.is_some());

    // Toggle off via admin endpoint
    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/admin/flash-sales/{}/toggle", session.id))
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Now active endpoint returns None
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/flash-sales/active")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let body = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let active_after: Option<FlashSaleSessionDto> = serde_json::from_slice(&body).unwrap();
    assert!(active_after.is_none());
}
