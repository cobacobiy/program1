use std::sync::Arc;
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use program1_contracts::{
    AuthContract, CatalogContract, ChannelType, OrderContract,
    SalesReportResponse, StorefrontOrderItemRequest, UserContract,
};
use program1_core::init_database;
use program1_module_analytics::AnalyticsModule;
use program1_module_audit::AuditModule;
use program1_module_auth::AuthModule;
use program1_module_buyer::BuyerModule;
use program1_module_catalog::CatalogModule;
use program1_module_channel::ChannelSyncModule;
use program1_module_chat::ChatModule;
use program1_module_coupon::CouponModule;
use program1_module_inventory::InventoryModule;
use program1_module_order::OrderModule;
use program1_module_payment::PaymentModule;
use program1_module_review::ReviewModule;
use program1_module_shipping::ShippingModule;
use program1_module_user::UserModule;
use program1_web::{create_app, AppState};
use tower::ServiceExt;
use uuid::Uuid;

struct TestContext {
    app: axum::Router,
    admin_token: String,
    buyer_token: String,
    catalog: Arc<CatalogModule>,
    order: Arc<OrderModule>,
    pool: program1_core::database::DbPool,
}

async fn setup_test_app() -> TestContext {
    let pool = init_database("sqlite::memory:").await.unwrap();

    let user_module = Arc::new(UserModule::new(pool.clone()));
    let auth_module = Arc::new(AuthModule::new(
        "super-secret-program1-jwt-signing-key-32chars-min!".to_string(),
        24,
    ));
    let catalog_module = Arc::new(CatalogModule::new(pool.clone()));
    let inventory_module = Arc::new(InventoryModule::new(
        pool.clone(),
        catalog_module.clone(),
    ));
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
        "".to_string(),
        "".to_string(),
        false,
    ));
    let coupon_module = Arc::new(CouponModule::new(pool.clone()));
    let review_module = Arc::new(ReviewModule::new(pool.clone()));
    let shipping_module = Arc::new(ShippingModule::new(
        "".to_string(),
        "starter".to_string(),
        "152".to_string(),
    ));

    let _ = user_module.seed_default_users().await;
    let _ = catalog_module.seed_default_catalog().await;
    let _ = channel_module.seed_default_channels().await;

    let state = AppState {
        store_name: "Aura Report Test Store".to_string(),
        store_currency: "IDR".to_string(),
        store_whatsapp_number: "08123456789".to_string(),
        user_contract: user_module.clone(),
        auth_contract: auth_module.clone(),
        catalog_contract: catalog_module.clone(),
        inventory_contract: inventory_module,
        channel_contract: channel_module,
        order_contract: order_module.clone(),
        analytics_contract: analytics_module,
        audit_contract: audit_module,
        buyer_contract: buyer_module,
        chat_contract: chat_module,
        payment_contract: payment_module,
        coupon_contract: coupon_module,
        review_contract: review_module,
        shipping_contract: shipping_module,
        rate_limiter: Arc::new(program1_web::rate_limit::IpRateLimiter::new()),
        started_at: std::time::Instant::now(),
        google_client_id: "test".to_string(),
    };

    // 1. Admin Token
    let admin_user = user_module.authenticate("admin", "admin123").await.unwrap();
    let admin_token = auth_module.generate_token(&admin_user).unwrap();

    // 2. Buyer Token
    let buyer_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let now_str = now.to_rfc3339();
    sqlx::query(
        "INSERT INTO buyer_accounts (id, google_sub, email, full_name, avatar_url, phone_number, phone_verified, is_active, created_at, updated_at)
         VALUES ($1, 'sub_report', 'report@buyer.com', 'Report Tester', NULL, '+6281234567890', 1, 1, $2, $3)",
    )
    .bind(buyer_id.to_string())
    .bind(&now_str)
    .bind(&now_str)
    .execute(&pool)
    .await
    .unwrap();

    let buyer_dto = program1_contracts::BuyerAccountDto {
        id: buyer_id,
        google_sub: Some("sub_report".to_string()),
        email: "report@buyer.com".to_string(),
        full_name: "Report Tester".to_string(),
        avatar_url: None,
        phone_number: Some("+6281234567890".to_string()),
        phone_verified: true,
        is_active: true,
        created_at: now,
        updated_at: now,
    };
    let buyer_token = auth_module.generate_buyer_token(&buyer_dto).unwrap();

    TestContext {
        app: create_app(state),
        admin_token,
        buyer_token,
        catalog: catalog_module,
        order: order_module,
        pool,
    }
}

#[tokio::test]
async fn test_sales_report_with_date_filter() {
    let ctx = setup_test_app().await;
    let items = ctx.catalog.list_items().await.unwrap();

    // Create Order 1 (current date)
    let order1 = ctx
        .order
        .create_marketplace_order(
            ChannelType::NativeWeb,
            "Buyer Today".to_string(),
            vec![StorefrontOrderItemRequest {
                product_id: items[0].id,
                quantity: 1,
            }],
        )
        .await
        .unwrap();

    // Create Order 2 (backdated to 2024-01-15 in SQLite)
    let order2 = ctx
        .order
        .create_marketplace_order(
            ChannelType::Shopee,
            "Buyer Past".to_string(),
            vec![StorefrontOrderItemRequest {
                product_id: items[1].id,
                quantity: 3,
            }],
        )
        .await
        .unwrap();

    sqlx::query("UPDATE orders SET created_at = '2024-01-15T10:00:00Z' WHERE id = $1")
        .bind(order2.id.to_string())
        .execute(&ctx.pool)
        .await
        .unwrap();

    // 1. Query for today's date range only
    let today_str = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let req_today = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/v1/analytics/report?date_from={}&date_to={}",
            today_str, today_str
        ))
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
        .body(Body::empty())
        .unwrap();

    let res_today = ctx.app.clone().oneshot(req_today).await.unwrap();
    assert_eq!(res_today.status(), StatusCode::OK);

    let body_today = axum::body::to_bytes(res_today.into_body(), usize::MAX).await.unwrap();
    let report_today: SalesReportResponse = serde_json::from_slice(&body_today).unwrap();
    assert_eq!(report_today.summary.total_orders, 1);
    assert_eq!(report_today.rows[0].order_id, order1.id.to_string());
    assert_eq!(report_today.rows[0].buyer_name, "Buyer Today");

    // 2. Query for 2024 date range only
    let req_past = Request::builder()
        .method("GET")
        .uri("/api/v1/analytics/report?date_from=2024-01-01&date_to=2024-01-31")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
        .body(Body::empty())
        .unwrap();

    let res_past = ctx.app.clone().oneshot(req_past).await.unwrap();
    assert_eq!(res_past.status(), StatusCode::OK);

    let body_past = axum::body::to_bytes(res_past.into_body(), usize::MAX).await.unwrap();
    let report_past: SalesReportResponse = serde_json::from_slice(&body_past).unwrap();
    assert_eq!(report_past.summary.total_orders, 1);
    assert_eq!(report_past.rows[0].order_id, order2.id.to_string());
    assert_eq!(report_past.rows[0].buyer_name, "Buyer Past");
    assert_eq!(report_past.summary.total_items_sold, 3);
}

#[tokio::test]
async fn test_sales_report_summary_calculation() {
    let ctx = setup_test_app().await;
    let items = ctx.catalog.list_items().await.unwrap();

    // Create 2 orders
    let _o1 = ctx
        .order
        .create_marketplace_order(
            ChannelType::NativeWeb,
            "Buyer A".to_string(),
            vec![StorefrontOrderItemRequest {
                product_id: items[0].id,
                quantity: 2,
            }],
        )
        .await
        .unwrap();

    let o2 = ctx
        .order
        .create_marketplace_order(
            ChannelType::TikTokShop,
            "Buyer B".to_string(),
            vec![StorefrontOrderItemRequest {
                product_id: items[1].id,
                quantity: 1,
            }],
        )
        .await
        .unwrap();

    // Update o2 to DELIVERED in DB
    sqlx::query("UPDATE orders SET status = 'DELIVERED' WHERE id = $1")
        .bind(o2.id.to_string())
        .execute(&ctx.pool)
        .await
        .unwrap();

    // Query all report
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/analytics/report")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let report: SalesReportResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(report.summary.total_orders, 2);
    assert_eq!(report.summary.total_items_sold, 3);
    assert!(report.summary.total_revenue_cents > 0);
    assert_eq!(
        report.summary.average_order_value_cents,
        report.summary.total_revenue_cents / 2
    );
    assert_eq!(*report.summary.orders_by_status.get("processing").unwrap(), 1);
    assert_eq!(*report.summary.orders_by_status.get("DELIVERED").unwrap(), 1);

    // Filter by delivered status
    let req_delivered = Request::builder()
        .method("GET")
        .uri("/api/v1/analytics/report?status_filter=delivered")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
        .body(Body::empty())
        .unwrap();

    let res_del = ctx.app.clone().oneshot(req_delivered).await.unwrap();
    assert_eq!(res_del.status(), StatusCode::OK);
    let body_del = axum::body::to_bytes(res_del.into_body(), usize::MAX).await.unwrap();
    let report_del: SalesReportResponse = serde_json::from_slice(&body_del).unwrap();
    assert_eq!(report_del.summary.total_orders, 1);
    assert_eq!(report_del.rows[0].order_id, o2.id.to_string());
    assert_eq!(report_del.rows[0].status, "DELIVERED");
    assert_eq!(report_del.rows[0].payment_status, "paid");
}

#[tokio::test]
async fn test_sales_report_auth_protection() {
    let ctx = setup_test_app().await;

    // 1. Unauthenticated (no token) -> 401 Unauthorized
    let req_no_auth = Request::builder()
        .method("GET")
        .uri("/api/v1/analytics/report")
        .body(Body::empty())
        .unwrap();

    let res_no_auth = ctx.app.clone().oneshot(req_no_auth).await.unwrap();
    assert_eq!(res_no_auth.status(), StatusCode::UNAUTHORIZED);

    // 2. Buyer Token (not admin/staff) -> 403 Forbidden
    let req_buyer = Request::builder()
        .method("GET")
        .uri("/api/v1/analytics/report")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.buyer_token))
        .body(Body::empty())
        .unwrap();

    let res_buyer = ctx.app.clone().oneshot(req_buyer).await.unwrap();
    assert_eq!(res_buyer.status(), StatusCode::FORBIDDEN);

    // 3. Admin Token -> 200 OK
    let req_admin = Request::builder()
        .method("GET")
        .uri("/api/v1/analytics/report")
        .header(header::AUTHORIZATION, format!("Bearer {}", ctx.admin_token))
        .body(Body::empty())
        .unwrap();

    let res_admin = ctx.app.oneshot(req_admin).await.unwrap();
    assert_eq!(res_admin.status(), StatusCode::OK);
}
