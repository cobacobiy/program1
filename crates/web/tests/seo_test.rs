use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use program1_contracts::{CatalogContract, CreateCatalogItemRequest};
use program1_core::init_database;
use program1_module_analytics::AnalyticsModule;
use program1_module_auth::AuthModule;
use program1_module_catalog::CatalogModule;
use program1_module_channel::ChannelSyncModule;
use program1_module_inventory::InventoryModule;
use program1_module_order::OrderModule;
use program1_module_user::UserModule;
use program1_web::{create_app, AppState};
use tower::ServiceExt;
use uuid::Uuid;

async fn setup_test_app() -> (axum::Router, Uuid) {
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

    // Create a test product
    let product = catalog_module
        .create_item(CreateCatalogItemRequest {
            name: "Sepatu Sneaker Klasik".to_string(),
            sku: "SKU-SNEAKER-01".to_string(),
            category: "Footwear".to_string(),
            price: 250000.0,
            stock: 20,
            image_url: Some("https://example.com/sneaker.jpg".to_string()),
            description: Some("Sepatu sneaker kanvas nyaman untuk harian.".to_string()),
            weight_grams: 800,
            category_id: None,
        })
        .await
        .expect("Failed creating test product");

    let state = AppState {
        store_name: "Toko Sneaker Keren".to_string(),
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
        shipping_contract: Arc::new(program1_module_shipping::ShippingModule::new(
            "".to_string(),
            "starter".to_string(),
            "152".to_string(),
        )),
        notification_contract: Arc::new(program1_module_notification::NotificationModule::new(
            pool.clone(),
        )),
        return_contract: Arc::new(program1_module_return::ReturnModule::new(pool.clone())),
        backup_contract: Arc::new(program1_core::backup::BackupService::new(
            pool.clone(),
            "./target/test_backups",
        )),
        rate_limiter: Arc::new(program1_web::rate_limit::IpRateLimiter::new()),
        started_at: std::time::Instant::now(),
        google_client_id: "test".to_string(),
    };

    let app = create_app(state);
    (app, product.id)
}

#[tokio::test]
async fn test_robots_txt() {
    let (app, _) = setup_test_app().await;

    let req = Request::builder()
        .uri("/robots.txt")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/plain; charset=utf-8"
    );

    let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(body.contains("User-agent: *"));
    assert!(body.contains("Disallow: /admin"));
    assert!(body.contains("Sitemap: /sitemap.xml"));
}

#[tokio::test]
async fn test_sitemap_xml() {
    let (app, product_id) = setup_test_app().await;

    let req = Request::builder()
        .uri("/sitemap.xml")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/xml; charset=utf-8"
    );

    let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let xml = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(xml.contains("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">"));
    assert!(xml.contains("<loc>/</loc>"));
    assert!(xml.contains(&format!("<loc>/product/{}</loc>", product_id)));
    assert!(xml.contains("<priority>0.8</priority>"));
}

#[tokio::test]
async fn test_social_crawler_open_graph_meta() {
    let (app, product_id) = setup_test_app().await;

    // 1. Social crawler (WhatsApp bot) request -> returns HTML with Open Graph
    let req = Request::builder()
        .uri(format!("/product/{}", product_id))
        .method("GET")
        .header(
            header::USER_AGENT,
            "WhatsApp/2.21.12.21 A (compatible; crawler)",
        )
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp
        .headers()
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .contains("text/html"));

    let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let html = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(html.contains("<meta property=\"og:type\" content=\"product\">"));
    assert!(html.contains("<meta property=\"og:title\" content=\"Sepatu Sneaker Klasik | Toko Sneaker Keren\">"));
    assert!(html.contains("<meta property=\"og:image\" content=\"https://example.com/sneaker.jpg\">"));
    assert!(html.contains("<meta property=\"product:price:amount\" content=\"250000\">"));
    assert!(html.contains("<meta property=\"product:price:currency\" content=\"IDR\">"));
    assert!(html.contains("<meta name=\"twitter:card\" content=\"summary_large_image\">"));

    // 2. Normal browser request -> redirects 302 to SPA product hash
    let req = Request::builder()
        .uri(format!("/product/{}", product_id))
        .method("GET")
        .header(
            header::USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120.0",
        )
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FOUND);
    let location = resp.headers().get(header::LOCATION).unwrap().to_str().unwrap();
    assert_eq!(location, format!("/#product-{}", product_id));
}
