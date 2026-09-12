use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
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
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

async fn setup_test_app() -> (
    axum::Router,
    Arc<dyn BuyerContract>,
    Arc<dyn OrderContract>,
    Arc<dyn CatalogContract>,
    String, // buyer 1 token
    Uuid,   // buyer 1 id
    String, // buyer 2 token
    Uuid,   // buyer 2 id
) {
    let pool = init_database("sqlite::memory:").await.unwrap();

    let user_module = Arc::new(UserModule::new(pool.clone()));
    let auth_module = Arc::new(AuthModule::new(
        "super-secret-jwt-signing-key-for-tests-32chars!".to_string(),
        24,
    ));
    let catalog_module = Arc::new(CatalogModule::new(pool.clone()));
    let inventory_module = Arc::new(InventoryModule::new(pool.clone(), catalog_module.clone()));
    let channel_module = Arc::new(ChannelSyncModule::new(pool.clone()));
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

    let order_module = Arc::new(
        OrderModule::new(
            pool.clone(),
            catalog_module.clone(),
            inventory_module.clone(),
        )
        .with_buyer_contract(buyer_module.clone()),
    );

    let analytics_module = Arc::new(AnalyticsModule::new(
        catalog_module.clone(),
        order_module.clone(),
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

    catalog_module.seed_default_catalog().await.unwrap();

    let state = AppState {
        store_name: "AURA Storefront".to_string(),
        store_currency: "IDR".to_string(),
        store_whatsapp_number: "+6281234567890".to_string(),
        user_contract: user_module,
        auth_contract: auth_module.clone(),
        catalog_contract: catalog_module.clone(),
        inventory_contract: inventory_module,
        channel_contract: channel_module,
        order_contract: order_module.clone(),
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
        flash_sale_contract: flash_sale_module,
        rate_limiter: Arc::new(program1_web::rate_limit::IpRateLimiter::new()),
        started_at: std::time::Instant::now(),
        google_client_id: "test".to_string(),
    };

    let app = create_app(state);

    let b1 = buyer_module
        .register(RegisterBuyerRequest {
            email: "buyer1_loyalty@example.com".to_string(),
            password: "Password123!".to_string(),
            full_name: "Buyer One".to_string(),
        })
        .await
        .unwrap();

    let b2 = buyer_module
        .register(RegisterBuyerRequest {
            email: "buyer2_loyalty@example.com".to_string(),
            password: "Password123!".to_string(),
            full_name: "Buyer Two".to_string(),
        })
        .await
        .unwrap();

    (
        app,
        buyer_module,
        order_module,
        catalog_module,
        b1.access_token,
        b1.buyer.id,
        b2.access_token,
        b2.buyer.id,
    )
}

#[tokio::test]
async fn test_loyalty_earning_on_delivered_order() {
    let (_app, buyer_contract, order_contract, catalog_contract, _, buyer_id, _, _) =
        setup_test_app().await;

    let items = catalog_contract.list_items().await.unwrap();
    let product_id = items[0].id;

    let order = order_contract
        .create_storefront_order(StorefrontOrderRequest {
            customer_name: "Buyer One".to_string(),
            customer_email: "buyer1_loyalty@example.com".to_string(),
            shipping_address: "Jl. Sudirman No 1".to_string(),
            items: vec![StorefrontOrderItemRequest {
                product_id,
                quantity: 1,
            }],
            buyer_id: Some(buyer_id),
            shipping_snapshot: None,
            courier: None,
            shipping_cost_cents: None,
            use_points: None,
        })
        .await
        .unwrap();

    // Advance status to Delivered
    order_contract
        .update_order_status(order.id, OrderStatus::Paid, "system")
        .await
        .unwrap();
    order_contract
        .update_order_status(order.id, OrderStatus::Processing, "system")
        .await
        .unwrap();
    order_contract
        .update_order_status(order.id, OrderStatus::Shipped, "system")
        .await
        .unwrap();
    order_contract
        .update_order_status(order.id, OrderStatus::Delivered, "system")
        .await
        .unwrap();

    // Allow tokio task to finish
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let summary = buyer_contract.get_loyalty_summary(buyer_id).await.unwrap();
    // 1 point per 100 IDR
    let expected_points = (order.total_amount / 100.0).floor() as i64;
    assert!(expected_points > 0);
    assert_eq!(summary.points_balance, expected_points);
    assert!(!summary.ledgers.is_empty());
}

#[tokio::test]
async fn test_loyalty_point_redemption_at_checkout() {
    let (_app, buyer_contract, order_contract, catalog_contract, _, buyer_id, _, _) =
        setup_test_app().await;

    // Credit buyer with 50,000 points
    buyer_contract
        .credit_points(buyer_id, None, 50000, "Bonus Signup")
        .await
        .unwrap();

    let items = catalog_contract.list_items().await.unwrap();
    let product_id = items[0].id;

    // Checkout with use_points = true
    let order = order_contract
        .create_storefront_order(StorefrontOrderRequest {
            customer_name: "Buyer One".to_string(),
            customer_email: "buyer1_loyalty@example.com".to_string(),
            shipping_address: "Jl. Sudirman No 1".to_string(),
            items: vec![StorefrontOrderItemRequest {
                product_id,
                quantity: 1,
            }],
            buyer_id: Some(buyer_id),
            shipping_snapshot: None,
            courier: None,
            shipping_cost_cents: None,
            use_points: Some(true),
        })
        .await
        .unwrap();

    assert_eq!(order.points_redeemed, 50000);
    let original_item_price = items[0].price;
    assert_eq!(order.total_amount, original_item_price - 50000.0);

    // Verify buyer's points balance is deducted
    let profile = buyer_contract.get_buyer_profile(buyer_id).await.unwrap();
    assert_eq!(profile.points_balance, 0);
}

#[tokio::test]
async fn test_loyalty_rollback_on_cancelled_order() {
    let (_app, buyer_contract, order_contract, catalog_contract, _, buyer_id, _, _) =
        setup_test_app().await;

    // Credit buyer with 30,000 points
    buyer_contract
        .credit_points(buyer_id, None, 30000, "Initial Points")
        .await
        .unwrap();

    let items = catalog_contract.list_items().await.unwrap();
    let product_id = items[0].id;

    // Checkout using the 30,000 points
    let order = order_contract
        .create_storefront_order(StorefrontOrderRequest {
            customer_name: "Buyer One".to_string(),
            customer_email: "buyer1_loyalty@example.com".to_string(),
            shipping_address: "Jl. Sudirman No 1".to_string(),
            items: vec![StorefrontOrderItemRequest {
                product_id,
                quantity: 1,
            }],
            buyer_id: Some(buyer_id),
            shipping_snapshot: None,
            courier: None,
            shipping_cost_cents: None,
            use_points: Some(true),
        })
        .await
        .unwrap();

    assert_eq!(order.points_redeemed, 30000);

    // Cancel order
    order_contract
        .update_order_status(order.id, OrderStatus::Cancelled, "buyer")
        .await
        .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Points should be restored to buyer
    let profile = buyer_contract.get_buyer_profile(buyer_id).await.unwrap();
    assert_eq!(profile.points_balance, 30000);
}

#[tokio::test]
async fn test_loyalty_buyer_isolation_and_api() {
    let (app, buyer_contract, _, _, token1, buyer1_id, token2, _buyer2_id) =
        setup_test_app().await;

    // Credit 25,000 points to Buyer 1 only
    buyer_contract
        .credit_points(buyer1_id, None, 25000, "Buyer 1 Reward")
        .await
        .unwrap();

    // Buyer 1 accesses /api/v1/buyer/loyalty -> 25,000 points
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/buyer/loyalty")
        .header(header::AUTHORIZATION, format!("Bearer {}", token1))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let summary1: LoyaltySummaryDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(summary1.points_balance, 25000);

    // Buyer 2 accesses /api/v1/buyer/loyalty -> 0 points (isolated)
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/buyer/loyalty")
        .header(header::AUTHORIZATION, format!("Bearer {}", token2))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let summary2: LoyaltySummaryDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(summary2.points_balance, 0);
}
