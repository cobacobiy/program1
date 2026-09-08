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
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

async fn setup_test_app() -> (axum::Router, String, String) {
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

    let _ = catalog_module
        .create_item(program1_contracts::CreateCatalogItemRequest {
            name: "AURA Studio Pro Wireless Headset".to_string(),
            sku: "SKU-AURA-AUD01".to_string(),
            category: "Audio".to_string(),
            category_id: None,
            price: 2100000.0,
            stock: 25,
            image_url: Some("https://example.com/audio.jpg".to_string()),
            description: Some("Hi-Res Noise Cancelling Headphones".to_string()),
            weight_grams: 500,
        })
        .await;

    let _ = catalog_module
        .create_item(program1_contracts::CreateCatalogItemRequest {
            name: "AURA Pro Gaming Desk Mat".to_string(),
            sku: "SKU-AURA-MAT02".to_string(),
            category: "Accessories".to_string(),
            category_id: None,
            price: 250000.0,
            stock: 100,
            image_url: Some("https://example.com/mat.jpg".to_string()),
            description: Some("Smooth gliding micro-weave surface".to_string()),
            weight_grams: 500,
        })
        .await;

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
        rate_limiter: Arc::new(program1_web::rate_limit::IpRateLimiter::new()),
        started_at: std::time::Instant::now(),
        google_client_id: "test".to_string(),
    };

    let app = create_app(state);

    let seller_claims = JwtClaims {
        sub: Uuid::new_v4(),
        username: "admin_seller".to_string(),
        role: "Super Admin".to_string(),
        accessible_menus: vec!["all".to_string(), "customers".to_string()],
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp(),
        iat: chrono::Utc::now().timestamp(),
        user_type: "seller_staff".to_string(),
    };
    let seller_token = encode(
        &Header::default(),
        &seller_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap();

    let buyer_claims = JwtClaims {
        sub: Uuid::new_v4(),
        username: "buyer_user".to_string(),
        role: "buyer".to_string(),
        accessible_menus: vec![],
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp(),
        iat: chrono::Utc::now().timestamp(),
        user_type: "buyer".to_string(),
    };
    let buyer_token = encode(
        &Header::default(),
        &buyer_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap();

    (app, seller_token, buyer_token)
}

#[tokio::test]
async fn test_catalog_pagination_defaults() {
    let (app, _, _) = setup_test_app().await;

    let req = Request::builder()
        .uri("/api/v1/catalog")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&body).unwrap();

    assert!(val.is_object());
    assert_eq!(val["page"], 1);
    assert_eq!(val["page_size"], 20);
    assert!(val["total"].as_i64().unwrap() >= 5);
    assert_eq!(val["total_pages"], 1);
    let items = val["data"].as_array().unwrap();
    assert_eq!(items.len() as i64, val["total"].as_i64().unwrap());
}

#[tokio::test]
async fn test_catalog_pagination_custom_page_and_size() {
    let (app, _, _) = setup_test_app().await;

    // Page 1 with page_size = 2
    let req1 = Request::builder()
        .uri("/api/v1/catalog?page=1&page_size=2")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res1 = app.clone().oneshot(req1).await.unwrap();
    assert_eq!(res1.status(), StatusCode::OK);
    let body1 = to_bytes(res1.into_body(), usize::MAX).await.unwrap();
    let val1: Value = serde_json::from_slice(&body1).unwrap();

    assert_eq!(val1["page"], 1);
    assert_eq!(val1["page_size"], 2);
    assert!(val1["total"].as_i64().unwrap() >= 5);
    let items1 = val1["data"].as_array().unwrap();
    assert_eq!(items1.len(), 2);
    let total_pages = val1["total_pages"].as_i64().unwrap();
    assert!(total_pages >= 3);

    // Page 2 with page_size = 2
    let req2 = Request::builder()
        .uri("/api/v1/catalog?page=2&page_size=2")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res2 = app.clone().oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::OK);
    let body2 = to_bytes(res2.into_body(), usize::MAX).await.unwrap();
    let val2: Value = serde_json::from_slice(&body2).unwrap();

    assert_eq!(val2["page"], 2);
    assert_eq!(val2["page_size"], 2);
    let items2 = val2["data"].as_array().unwrap();
    assert_eq!(items2.len(), 2);

    // Items on page 1 and page 2 must be distinct
    assert_ne!(items1[0]["id"], items2[0]["id"]);
    assert_ne!(items1[1]["id"], items2[1]["id"]);
}

#[tokio::test]
async fn test_catalog_pagination_out_of_range() {
    let (app, _, _) = setup_test_app().await;

    let req = Request::builder()
        .uri("/api/v1/catalog?page=999&page_size=20")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(val["page"], 999);
    assert_eq!(val["page_size"], 20);
    assert!(val["total"].as_i64().unwrap() > 0);
    let items = val["data"].as_array().unwrap();
    assert_eq!(items.len(), 0);
}

#[tokio::test]
async fn test_catalog_search_and_category_filter() {
    let (app, _, _) = setup_test_app().await;

    // Search for "Pro"
    let req_search = Request::builder()
        .uri("/api/v1/catalog?search=Pro")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res_search = app.clone().oneshot(req_search).await.unwrap();
    assert_eq!(res_search.status(), StatusCode::OK);
    let body_search = to_bytes(res_search.into_body(), usize::MAX).await.unwrap();
    let val_search: Value = serde_json::from_slice(&body_search).unwrap();

    let items_search = val_search["data"].as_array().unwrap();
    assert!(!items_search.is_empty());
    for item in items_search {
        let name = item["name"].as_str().unwrap().to_lowercase();
        let sku = item["sku"].as_str().unwrap().to_lowercase();
        assert!(name.contains("pro") || sku.contains("pro"));
    }

    // Filter by Category "Audio"
    let req_cat = Request::builder()
        .uri("/api/v1/catalog?category=Audio")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res_cat = app.clone().oneshot(req_cat).await.unwrap();
    assert_eq!(res_cat.status(), StatusCode::OK);
    let body_cat = to_bytes(res_cat.into_body(), usize::MAX).await.unwrap();
    let val_cat: Value = serde_json::from_slice(&body_cat).unwrap();

    let items_cat = val_cat["data"].as_array().unwrap();
    assert!(!items_cat.is_empty());
    for item in items_cat {
        assert_eq!(item["category"].as_str().unwrap(), "Audio");
    }
}

#[tokio::test]
async fn test_catalog_sorting() {
    let (app, _, _) = setup_test_app().await;

    // Sort by price ASC
    let req_asc = Request::builder()
        .uri("/api/v1/catalog?sort_by=price&sort_order=asc")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res_asc = app.clone().oneshot(req_asc).await.unwrap();
    assert_eq!(res_asc.status(), StatusCode::OK);
    let body_asc = to_bytes(res_asc.into_body(), usize::MAX).await.unwrap();
    let val_asc: Value = serde_json::from_slice(&body_asc).unwrap();
    let items_asc = val_asc["data"].as_array().unwrap();
    assert!(items_asc.len() >= 2);

    let mut prev_price = 0.0;
    for item in items_asc {
        let price = item["price"].as_f64().unwrap();
        assert!(price >= prev_price);
        prev_price = price;
    }

    // Sort by price DESC
    let req_desc = Request::builder()
        .uri("/api/v1/catalog?sort_by=price&sort_order=desc")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res_desc = app.clone().oneshot(req_desc).await.unwrap();
    assert_eq!(res_desc.status(), StatusCode::OK);
    let body_desc = to_bytes(res_desc.into_body(), usize::MAX).await.unwrap();
    let val_desc: Value = serde_json::from_slice(&body_desc).unwrap();
    let items_desc = val_desc["data"].as_array().unwrap();
    assert!(items_desc.len() >= 2);

    let mut prev_price = f64::MAX;
    for item in items_desc {
        let price = item["price"].as_f64().unwrap();
        assert!(price <= prev_price);
        prev_price = price;
    }
}

#[tokio::test]
async fn test_catalog_pagination_validation_errors() {
    let (app, _, _) = setup_test_app().await;

    // page = 0 is invalid (min: 1)
    let req_zero_page = Request::builder()
        .uri("/api/v1/catalog?page=0")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req_zero_page).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // page_size = 101 is invalid (max: 100)
    let req_over_size = Request::builder()
        .uri("/api/v1/catalog?page_size=101")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req_over_size).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_orders_pagination_and_status_filter() {
    let (app, seller_token, _) = setup_test_app().await;

    // GET /api/v1/orders with pagination
    let req = Request::builder()
        .uri("/api/v1/orders?page=1&page_size=10")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", seller_token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert!(val.is_object());
    assert_eq!(val["page"], 1);
    assert_eq!(val["page_size"], 10);
    assert!(val["data"].is_array());

    // Filter by status "delivered"
    let req_status = Request::builder()
        .uri("/api/v1/orders?status=delivered")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", seller_token))
        .body(Body::empty())
        .unwrap();
    let res_status = app.clone().oneshot(req_status).await.unwrap();
    assert_eq!(res_status.status(), StatusCode::OK);
    let body_status = to_bytes(res_status.into_body(), usize::MAX).await.unwrap();
    let val_status: Value = serde_json::from_slice(&body_status).unwrap();
    for order in val_status["data"].as_array().unwrap() {
        assert_eq!(order["status"].as_str().unwrap(), "delivered");
    }
}

#[tokio::test]
async fn test_inventory_pagination_and_search() {
    let (app, seller_token, _) = setup_test_app().await;

    let req = Request::builder()
        .uri("/api/v1/inventory?page=1&page_size=3")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", seller_token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val["page"], 1);
    assert_eq!(val["page_size"], 3);
    assert!(val["total"].as_i64().unwrap() >= 5);
    assert_eq!(val["data"].as_array().unwrap().len(), 3);

    // Search inventory
    let req_search = Request::builder()
        .uri("/api/v1/inventory?search=Pro")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", seller_token))
        .body(Body::empty())
        .unwrap();
    let res_search = app.clone().oneshot(req_search).await.unwrap();
    assert_eq!(res_search.status(), StatusCode::OK);
    let body_search = to_bytes(res_search.into_body(), usize::MAX).await.unwrap();
    let val_search: Value = serde_json::from_slice(&body_search).unwrap();
    let items = val_search["data"].as_array().unwrap();
    assert!(!items.is_empty());
}

#[tokio::test]
async fn test_admin_buyers_pagination_and_search() {
    let (app, seller_token, _) = setup_test_app().await;

    let req = Request::builder()
        .uri("/api/v1/admin/buyers?page=1&page_size=5")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", seller_token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val["page"], 1);
    assert_eq!(val["page_size"], 5);
    assert!(val["data"].is_array());
}
