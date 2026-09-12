use std::sync::Arc;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use program1_contracts::{
    AuthContract, CatalogContract, ShippingCost, ShippingCourier, ShippingCity, UserContract,
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
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

async fn setup_test_app() -> (axum::Router, String, String, Uuid, Uuid) {
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
        store_name: "Aura Shipping Test Store".to_string(),
        store_currency: "IDR".to_string(),
        store_whatsapp_number: "08123456789".to_string(),
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
        notification_contract: Arc::new(program1_module_notification::NotificationModule::new(pool.clone())),
        return_contract: Arc::new(program1_module_return::ReturnModule::new(pool.clone())),
        backup_contract: Arc::new(program1_core::backup::BackupService::new(pool.clone(), "./target/test_backups")),
        rate_limiter: Arc::new(program1_web::rate_limit::IpRateLimiter::new()),
        started_at: std::time::Instant::now(),
        google_client_id: "test".to_string(),
    };

    // 1. Admin token
    let admin_user = user_module.authenticate("admin", "admin123").await.unwrap();
    let admin_token = auth_module.generate_token(&admin_user).unwrap();

    // 2. Buyer account & token
    let buyer_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let now_str = now.to_rfc3339();
    sqlx::query(
        "INSERT INTO buyer_accounts (id, google_sub, email, full_name, avatar_url, phone_number, phone_verified, is_active, created_at, updated_at)
         VALUES ($1, 'sub_val', 'buyer.shipping@test.com', 'Shipping Tester', NULL, '+6281234567890', 1, 1, $2, $3)",
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
        email: "buyer.shipping@test.com".to_string(),
        full_name: "Shipping Tester".to_string(),
        avatar_url: None,
        phone_number: Some("+6281234567890".to_string()),
        phone_verified: true,
        is_active: true,
        created_at: now,
        updated_at: now,
    };
    let buyer_token = auth_module.generate_buyer_token(&buyer_dto).unwrap();

    // 3. Add shipping address
    let addr_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO buyer_addresses (id, buyer_id, recipient_name, phone_number, street_address, subdistrict, city, province, postal_code, is_default, created_at, updated_at)
         VALUES ($1, $2, 'Shipping Tester', '+6281234567890', 'Jl. Pemuda No. 45', 'Gubeng', 'Surabaya', 'Jawa Timur', '60281', 1, $3, $4)",
    )
    .bind(addr_id.to_string())
    .bind(buyer_id.to_string())
    .bind(&now_str)
    .bind(&now_str)
    .execute(&pool)
    .await
    .unwrap();

    // 4. Get a seeded product id
    let items = catalog_module.list_items().await.unwrap();
    let product_id = items[0].id;

    (
        create_app(state),
        admin_token,
        buyer_token,
        addr_id,
        product_id,
    )
}

#[tokio::test]
async fn test_calculate_shipping_cost_api() {
    let (app, _, _, _, _) = setup_test_app().await;

    // 1. Calculate 500g shipping with JNE
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/shipping/cost")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "destination_city_id": "444",
                "weight_grams": 500,
                "courier": "jne"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let costs: Vec<ShippingCost> = serde_json::from_slice(&body).unwrap();
    assert!(!costs.is_empty());

    let reg = costs.iter().find(|c| c.service == "REG").unwrap();
    assert_eq!(reg.cost_cents, 18000);
    assert_eq!(reg.courier_code, "jne");

    // 2. Calculate 1500g shipping (2 kg bracket)
    let req_2kg = Request::builder()
        .method("POST")
        .uri("/api/v1/shipping/cost")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "destination_city_id": "444",
                "weight_grams": 1500,
                "courier": "jne"
            })
            .to_string(),
        ))
        .unwrap();

    let res_2kg = app.oneshot(req_2kg).await.unwrap();
    assert_eq!(res_2kg.status(), StatusCode::OK);
    let body_2kg = axum::body::to_bytes(res_2kg.into_body(), usize::MAX).await.unwrap();
    let costs_2kg: Vec<ShippingCost> = serde_json::from_slice(&body_2kg).unwrap();
    let reg_2kg = costs_2kg.iter().find(|c| c.service == "REG").unwrap();
    assert_eq!(reg_2kg.cost_cents, 36000);
}

#[tokio::test]
async fn test_list_couriers_and_cities_api() {
    let (app, _, _, _, _) = setup_test_app().await;

    // 1. Couriers
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/shipping/couriers")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let couriers: Vec<ShippingCourier> = serde_json::from_slice(&body).unwrap();
    assert!(couriers.iter().any(|c| c.code == "jne"));
    assert!(couriers.iter().any(|c| c.code == "tiki"));
    assert!(couriers.iter().any(|c| c.code == "pos"));

    // 2. Cities search autocomplete
    let req_city = Request::builder()
        .method("GET")
        .uri("/api/v1/shipping/cities?q=Jakarta")
        .body(Body::empty())
        .unwrap();

    let res_city = app.oneshot(req_city).await.unwrap();
    assert_eq!(res_city.status(), StatusCode::OK);
    let body_city = axum::body::to_bytes(res_city.into_body(), usize::MAX).await.unwrap();
    let cities: Vec<ShippingCity> = serde_json::from_slice(&body_city).unwrap();
    assert!(!cities.is_empty());
    assert!(cities.iter().any(|c| c.city_name.contains("Jakarta")));
}

#[tokio::test]
async fn test_checkout_with_shipping_cost() {
    let (app, admin_token, buyer_token, address_id, product_id) = setup_test_app().await;

    // Place buyer storefront checkout with shipping option
    let shipping_fee = 18000;
    let checkout_req = Request::builder()
        .method("POST")
        .uri("/api/v1/orders")
        .header("Authorization", format!("Bearer {}", buyer_token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "address_id": address_id.to_string(),
                "items": [
                    {
                        "product_id": product_id.to_string(),
                        "quantity": 1
                    }
                ],
                "courier": "JNE - REG",
                "shipping_cost_cents": shipping_fee
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(checkout_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let order: Value = serde_json::from_slice(&body).unwrap();
    let order_id = order["id"].as_str().unwrap();

    assert_eq!(order["courier"], "JNE - REG");
    assert_eq!(order["shipping_cost_cents"], shipping_fee);
    let subtotal = order["items"][0]["total_price"].as_f64().unwrap();
    let expected_total = subtotal + (shipping_fee as f64);
    assert_eq!(order["total_amount"].as_f64().unwrap(), expected_total);

    // 3. Admin updates tracking resi number
    let track_req = Request::builder()
        .method("PATCH")
        .uri(format!("/api/v1/orders/{}/tracking", order_id))
        .header("Authorization", format!("Bearer {}", admin_token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "tracking_number": "JNE-RESI-998877"
            })
            .to_string(),
        ))
        .unwrap();

    let track_res = app.clone().oneshot(track_req).await.unwrap();
    assert_eq!(track_res.status(), StatusCode::OK);

    let track_body = axum::body::to_bytes(track_res.into_body(), usize::MAX).await.unwrap();
    let updated_order: Value = serde_json::from_slice(&track_body).unwrap();
    assert_eq!(updated_order["tracking_number"], "JNE-RESI-998877");

    // 4. Buyer cannot update tracking number (403 Forbidden)
    let buyer_track_req = Request::builder()
        .method("PATCH")
        .uri(format!("/api/v1/orders/{}/tracking", order_id))
        .header("Authorization", format!("Bearer {}", buyer_token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "tracking_number": "HACK-RESI-000"
            })
            .to_string(),
        ))
        .unwrap();

    let buyer_track_res = app.oneshot(buyer_track_req).await.unwrap();
    assert_eq!(buyer_track_res.status(), StatusCode::FORBIDDEN);
}
