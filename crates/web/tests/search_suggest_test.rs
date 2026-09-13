use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
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
use program1_module_inventory::InventoryModule;
use program1_module_order::OrderModule;
use program1_module_payment::PaymentModule;
use program1_module_review::ReviewModule;
use program1_module_shipping::ShippingModule;
use program1_module_user::UserModule;
use program1_web::{create_app, state::AppState};
use std::sync::Arc;
use tower::ServiceExt;

async fn setup_test_app() -> (axum::Router, Arc<dyn CatalogContract>) {
    let pool = init_database("sqlite::memory:").await.unwrap();

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

    catalog_module.seed_default_catalog().await.unwrap();

    let state = AppState {
        store_name: "AURA Storefront".to_string(),
        store_currency: "IDR".to_string(),
        store_whatsapp_number: "+6281234567890".to_string(),
        user_contract: user_module,
        auth_contract: auth_module,
        catalog_contract: catalog_module.clone(),
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
        notification_contract: Arc::new(program1_module_notification::NotificationModule::new(
            pool.clone(),
        )),
        return_contract: Arc::new(program1_module_return::ReturnModule::new(pool.clone())),
        backup_contract: Arc::new(program1_core::backup::BackupService::new(
            pool.clone(),
            "./target/test_backups",
        )),
        flash_sale_contract: Arc::new(program1_module_flash_sale::FlashSaleModule::new(pool.clone())),
        supplier_contract: Arc::new(program1_module_supplier::SupplierModule::new(pool.clone())),
        rate_limiter: Arc::new(program1_web::rate_limit::IpRateLimiter::new()),
        started_at: std::time::Instant::now(),
        google_client_id: "test".to_string(),
    };

    let app = create_app(state);
    (app, catalog_module)
}

#[tokio::test]
async fn test_search_suggest_prefix_match() {
    let (app, catalog) = setup_test_app().await;

    // Seed extra item
    let _ = catalog
        .create_item(CreateCatalogItemRequest {
            name: "Sepatu Sneaker Olahraga Aerostreet".to_string(),
            sku: "SKU-SEPATU-01".to_string(),
            category: "Fashion".to_string(),
            price: 250000.0,
            stock: 50,
            image_url: Some("".to_string()),
            description: Some("Sepatu sneaker pria wanita nyaman untuk lari dan santai".to_string()),
            weight_grams: 800,
            category_id: None,
        })
        .await
        .unwrap();

    // Query "sepa"
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/catalog/suggest?q=sepa&limit=5")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let result: SearchSuggestionResult = serde_json::from_slice(&body).unwrap();

    assert_eq!(result.query, "sepa");
    assert!(!result.product_suggestions.is_empty());
    assert!(result.product_suggestions[0].name.contains("Sepatu"));
}

fn url_encode(input: &str) -> String {
    let mut encoded = String::new();
    for b in input.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            encoded.push(b as char);
        } else {
            encoded.push_str(&format!("%{:02X}", b));
        }
    }
    encoded
}

#[tokio::test]
async fn test_search_suggest_special_characters_sanitization() {
    let (app, _catalog) = setup_test_app().await;

    // FTS query with tricky characters: quotes, asterisks, boolean words, operators
    let malicious_queries = vec![
        r#"AURA" OR 1=1"#,
        r#"*keyboard* AND NOT (broken)"#,
        r#""unclosed quotes"#,
        r#"^keyboard:*:*"#,
        r#"   "#,
    ];

    for q in malicious_queries {
        let encoded = url_encode(q);
        let req = Request::builder()
            .method("GET")
            .uri(format!("/api/v1/catalog/suggest?q={}", encoded))
            .body(Body::empty())
            .unwrap();

        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "FTS failed on query: {}",
            q
        );

        let body = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let result: SearchSuggestionResult = serde_json::from_slice(&body).unwrap();
        assert_eq!(result.query, q);
    }
}

#[tokio::test]
async fn test_popular_searches_tracking_and_api() {
    let (app, catalog) = setup_test_app().await;

    catalog.record_search_query("keyboard mechanical").await.unwrap();
    catalog.record_search_query("keyboard mechanical").await.unwrap();
    catalog.record_search_query("mouse wireless").await.unwrap();

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/catalog/popular-searches?limit=10")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let popular: Vec<PopularSearchKeyword> = serde_json::from_slice(&body).unwrap();

    assert!(!popular.is_empty());
    assert_eq!(popular[0].keyword, "keyboard mechanical");
    assert!(popular[0].search_count >= 2);
}
