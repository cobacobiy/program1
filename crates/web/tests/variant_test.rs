use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use jsonwebtoken::{encode, EncodingKey, Header};
use program1_contracts::{CatalogContract, JwtClaims};
use program1_core::init_database;
use program1_module_analytics::AnalyticsModule;
use program1_module_auth::AuthModule;
use program1_module_catalog::CatalogModule;
use program1_module_channel::ChannelSyncModule;
use program1_module_inventory::InventoryModule;
use program1_module_order::OrderModule;
use program1_module_user::UserModule;
use program1_web::{create_app, AppState};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

async fn setup_test_app() -> (axum::Router, String, Uuid) {
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
    let coupon_module = Arc::new(program1_module_coupon::CouponModule::new(pool.clone()));
    let review_module = Arc::new(program1_module_review::ReviewModule::new(pool.clone()));

    let _ = user_module.seed_default_users().await;
    let _ = catalog_module.seed_default_catalog().await;
    let _ = channel_module.seed_default_channels().await;

    let test_item = catalog_module
        .create_item(program1_contracts::CreateCatalogItemRequest {
            name: "AURA Pro Mechanical Keyboard TKL".to_string(),
            sku: "SKU-AURA-TKL99".to_string(),
            category: "Peripherals".to_string(),
            category_id: None,
            price: 1200000.0,
            stock: 50,
            image_url: Some("https://example.com/tkl.jpg".to_string()),
            description: Some("Tenkeyless layout with RGB switches".to_string()),
            weight_grams: 500,
        })
        .await
        .expect("Failed to create test catalog item");

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
        coupon_contract: coupon_module,
        review_contract: review_module,
        shipping_contract: Arc::new(program1_module_shipping::ShippingModule::new("".to_string(), "starter".to_string(), "152".to_string())),
        notification_contract: Arc::new(program1_module_notification::NotificationModule::new(pool.clone())),
        return_contract: Arc::new(program1_module_return::ReturnModule::new(pool.clone())),
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
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp(),
        iat: chrono::Utc::now().timestamp(),
        user_type: "seller_staff".to_string(),
    };
    let token = encode(
        &Header::default(),
        &seller_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("Failed to encode seller test token");

    (app, token, test_item.id)
}

#[tokio::test]
async fn test_create_and_list_variants() {
    let (app, token, product_id) = setup_test_app().await;

    // 1. Create Variant S
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/catalog/{}/variants", product_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "variant_name": "Ukuran",
                "variant_value": "S",
                "sku": "SKU-TKL-S",
                "price_override": null,
                "stock_quantity": 15
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let v_s: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v_s["variant_name"], "Ukuran");
    assert_eq!(v_s["variant_value"], "S");
    assert_eq!(v_s["stock_quantity"], 15);
    let s_id = v_s["id"].as_str().unwrap().to_string();

    // 2. Create Variant M with Price Override
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/catalog/{}/variants", product_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "variant_name": "Ukuran",
                "variant_value": "M",
                "sku": "SKU-TKL-M",
                "price_override": 1350000.0,
                "stock_quantity": 20
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // 3. Create Variant L
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/catalog/{}/variants", product_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "variant_name": "Ukuran",
                "variant_value": "L",
                "sku": "SKU-TKL-L",
                "price_override": 1500000.0,
                "stock_quantity": 25
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // 4. Public List variants -> should have 3
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/catalog/{}/variants", product_id))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let list: Vec<Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(list.len(), 3);

    // 5. Update Variant S
    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/catalog/{}/variants/{}", product_id, s_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "variant_name": "Ukuran",
                "variant_value": "Small-Slim",
                "sku": "SKU-TKL-S-SLIM",
                "price_override": 1250000.0,
                "stock_quantity": 30,
                "is_active": true
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let updated: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(updated["variant_value"], "Small-Slim");
    assert_eq!(updated["stock_quantity"], 30);

    // 6. Delete Variant S
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/catalog/{}/variants/{}", product_id, s_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // 7. Verify 2 variants remain
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/catalog/{}/variants", product_id))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let list_after: Vec<Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(list_after.len(), 2);
}

#[tokio::test]
async fn test_variant_price_override() {
    let (app, token, product_id) = setup_test_app().await;

    // Create variant with price override
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/catalog/{}/variants", product_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "variant_name": "Switch Type",
                "variant_value": "Linear Red",
                "sku": "SKU-SW-RED",
                "price_override": 1499000.0,
                "stock_quantity": 40
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let created: Value = serde_json::from_slice(&body).unwrap();
    let vid = created["id"].as_str().unwrap();

    // Fetch single variant
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/catalog/{}/variants/{}", product_id, vid))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let fetched: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(fetched["price_override"], 1499000.0);
    assert_eq!(fetched["variant_value"], "Linear Red");
}

#[tokio::test]
async fn test_duplicate_variant_rejected() {
    let (app, token, product_id) = setup_test_app().await;

    // Create first variant
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/catalog/{}/variants", product_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "variant_name": "Color",
                "variant_value": "Titanium Grey",
                "sku": null,
                "price_override": null,
                "stock_quantity": 10
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // Try creating identical variant
    let req_dup = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/catalog/{}/variants", product_id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "variant_name": "Color",
                "variant_value": "Titanium Grey",
                "sku": null,
                "price_override": null,
                "stock_quantity": 5
            })
            .to_string(),
        ))
        .unwrap();

    let res_dup = app.clone().oneshot(req_dup).await.unwrap();
    assert_eq!(res_dup.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_variant_unauthorized_mutation() {
    let (app, _token, product_id) = setup_test_app().await;

    // Try POST without Authorization header
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/catalog/{}/variants", product_id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "variant_name": "Ukuran",
                "variant_value": "XL",
                "stock_quantity": 10
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
