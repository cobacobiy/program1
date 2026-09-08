use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use jsonwebtoken::{encode, EncodingKey, Header};
use program1_contracts::{CatalogItemDto, CategoryDto, JwtClaims, PaginatedResponse};
use program1_core::init_database;
use program1_module_analytics::AnalyticsModule;
use program1_module_auth::AuthModule;
use program1_module_catalog::CatalogModule;
use program1_module_channel::ChannelSyncModule;
use program1_module_inventory::InventoryModule;
use program1_module_order::OrderModule;
use program1_module_user::UserModule;
use program1_web::{create_app, AppState};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

async fn setup_test_app() -> (axum::Router, String) {
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

    (app, token)
}

#[tokio::test]
async fn test_list_default_seeded_categories() {
    let (app, _) = setup_test_app().await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/categories")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let categories: Vec<CategoryDto> = serde_json::from_slice(&body).unwrap();

    assert!(!categories.is_empty());
    assert!(categories.iter().any(|c| c.name == "Peripherals"));
    assert!(categories.iter().any(|c| c.slug == "peripherals"));
}

#[tokio::test]
async fn test_create_and_get_category() {
    let (app, token) = setup_test_app().await;

    // 1. Create Category
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/categories")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "name": "Audio & Sound",
                "slug": "audio-sound",
                "description": "Headphones and DACs",
                "icon": "🎧",
                "sort_order": 15
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let created: CategoryDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(created.name, "Audio & Sound");
    assert_eq!(created.slug, "audio-sound");
    assert_eq!(created.icon.as_deref(), Some("🎧"));

    // 2. Get Category by ID
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/categories/{}", created.id))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 3. Get Category by Slug
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/categories/audio-sound")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_update_category() {
    let (app, token) = setup_test_app().await;

    // 1. Create Category
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/categories")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "name": "Smart Home",
                "slug": "smart-home",
                "description": "IoT gadgets",
                "icon": "🏠",
                "sort_order": 20
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let created: CategoryDto = serde_json::from_slice(&body).unwrap();

    // 2. Update Category
    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/admin/categories/{}", created.id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "name": "Smart Home & IoT",
                "slug": "smart-home-iot",
                "description": "IoT devices and sensors",
                "icon": "💡",
                "sort_order": 22,
                "is_active": true
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let updated: CategoryDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(updated.name, "Smart Home & IoT");
    assert_eq!(updated.slug, "smart-home-iot");
    assert_eq!(updated.icon.as_deref(), Some("💡"));
}

#[tokio::test]
async fn test_filter_catalog_by_category() {
    let (app, _) = setup_test_app().await;

    // Filter by category name
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/catalog?category=Peripherals")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let response: PaginatedResponse<CatalogItemDto> = serde_json::from_slice(&body).unwrap();
    assert!(!response.data.is_empty());
    for item in &response.data {
        assert_eq!(item.category, "Peripherals");
    }

    // Filter by slug
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/catalog?category=peripherals")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let response_slug: PaginatedResponse<CatalogItemDto> = serde_json::from_slice(&body).unwrap();
    assert!(!response_slug.data.is_empty());
}

#[tokio::test]
async fn test_category_delete_guard() {
    let (app, token) = setup_test_app().await;

    // Attempt to delete category that has products (Peripherals -> cat-002)
    let req = Request::builder()
        .method("DELETE")
        .uri("/api/v1/admin/categories/cat-002")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Create a new empty category and delete it
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/categories")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(
            json!({
                "name": "Empty Category",
                "slug": "empty-category"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let cat: CategoryDto = serde_json::from_slice(&body).unwrap();

    // Delete the empty category
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/admin/categories/{}", cat.id))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Verify it's gone
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/categories/{}", cat.id))
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_category_admin_authorization() {
    let (app, _) = setup_test_app().await;

    // Unauthorized category creation
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/categories")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "name": "Unauthorized Cat",
                "slug": "unauthorized-cat"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
