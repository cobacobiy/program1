use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use program1_contracts::{CatalogContract, JwtClaims, WishlistItemDto};
use program1_core::init_database;
use program1_module_analytics::AnalyticsModule;
use program1_module_audit::AuditModule;
use program1_module_auth::AuthModule;
use program1_module_catalog::CatalogModule;
use program1_module_channel::ChannelSyncModule;
use program1_module_coupon::CouponModule;
use program1_module_inventory::InventoryModule;
use program1_module_order::OrderModule;
use program1_module_payment::PaymentModule;
use program1_module_review::ReviewModule;
use program1_module_user::UserModule;
use program1_web::{create_app, AppState};
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

struct TestFixture {
    app: axum::Router,
    buyer1_token: String,
    buyer1_id: Uuid,
    buyer2_token: String,
    _buyer2_id: Uuid,
    product1_id: Uuid,
    product2_id: Uuid,
}

async fn setup_wishlist_test() -> TestFixture {
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
    let audit_module = Arc::new(AuditModule::new(pool.clone()));

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
    let payment_module = Arc::new(PaymentModule::new(
        pool.clone(),
        "test-server-key".to_string(),
        "test-client-key".to_string(),
        false,
    ));
    let coupon_module = Arc::new(CouponModule::new(pool.clone()));
    let review_module = Arc::new(ReviewModule::new(pool.clone()));

    let _ = user_module.seed_default_users().await;
    let _ = catalog_module.seed_default_catalog().await;
    let _ = channel_module.seed_default_channels().await;

    let state = AppState {
        store_name: "Wishlist Test Store".to_string(),
        store_currency: "IDR".to_string(),
        store_whatsapp_number: "08123456789".to_string(),
        user_contract: user_module,
        auth_contract: auth_module.clone(),
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
        shipping_contract: Arc::new(program1_module_shipping::ShippingModule::new("".to_string(), "starter".to_string(), "152".to_string())),
        rate_limiter: Arc::new(program1_web::rate_limit::IpRateLimiter::new()),
        started_at: std::time::Instant::now(),
        google_client_id: "test".to_string(),
    };

    let app = create_app(state);

    // Create Buyer 1 in DB
    let buyer1_id = Uuid::new_v4();
    let now_str = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO buyer_accounts (id, email, full_name, phone_number, phone_verified, is_active, created_at, updated_at)
         VALUES ($1, 'buyer1@example.com', 'Buyer One', '+62811111111', 1, 1, $2, $3)",
    )
    .bind(buyer1_id.to_string())
    .bind(&now_str)
    .bind(&now_str)
    .execute(&pool)
    .await
    .unwrap();

    let buyer1_claims = JwtClaims {
        sub: buyer1_id,
        username: "buyer1@example.com".to_string(),
        role: "buyer".to_string(),
        accessible_menus: vec!["buyer_storefront".to_string()],
        exp: (Utc::now() + Duration::hours(1)).timestamp(),
        iat: Utc::now().timestamp(),
        user_type: "buyer".to_string(),
    };
    let buyer1_token = encode(
        &Header::default(),
        &buyer1_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap();

    // Create Buyer 2 in DB
    let buyer2_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO buyer_accounts (id, email, full_name, phone_number, phone_verified, is_active, created_at, updated_at)
         VALUES ($1, 'buyer2@example.com', 'Buyer Two', '+62822222222', 1, 1, $2, $3)",
    )
    .bind(buyer2_id.to_string())
    .bind(&now_str)
    .bind(&now_str)
    .execute(&pool)
    .await
    .unwrap();

    let buyer2_claims = JwtClaims {
        sub: buyer2_id,
        username: "buyer2@example.com".to_string(),
        role: "buyer".to_string(),
        accessible_menus: vec!["buyer_storefront".to_string()],
        exp: (Utc::now() + Duration::hours(1)).timestamp(),
        iat: Utc::now().timestamp(),
        user_type: "buyer".to_string(),
    };
    let buyer2_token = encode(
        &Header::default(),
        &buyer2_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap();

    // Fetch two seeded catalog items
    let catalog_items = catalog_module.list_items().await.unwrap();
    let product1_id = catalog_items[0].id;
    let product2_id = catalog_items[1].id;

    TestFixture {
        app,
        buyer1_token,
        buyer1_id,
        buyer2_token,
        _buyer2_id: buyer2_id,
        product1_id,
        product2_id,
    }
}

#[tokio::test]
async fn test_unauthorized_access_to_wishlist() {
    let fixture = setup_wishlist_test().await;

    // GET /api/v1/buyer/wishlist without token
    let req = Request::builder()
        .uri("/api/v1/buyer/wishlist")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = fixture.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // POST /api/v1/buyer/wishlist/:id without token
    let req = Request::builder()
        .uri(format!("/api/v1/buyer/wishlist/{}", fixture.product1_id))
        .method("POST")
        .body(Body::empty())
        .unwrap();
    let resp = fixture.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // DELETE /api/v1/buyer/wishlist/:id without token
    let req = Request::builder()
        .uri(format!("/api/v1/buyer/wishlist/{}", fixture.product1_id))
        .method("DELETE")
        .body(Body::empty())
        .unwrap();
    let resp = fixture.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // GET /api/v1/buyer/wishlist/:id/check without token
    let req = Request::builder()
        .uri(format!(
            "/api/v1/buyer/wishlist/{}/check",
            fixture.product1_id
        ))
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = fixture.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_add_and_list_wishlist() {
    let fixture = setup_wishlist_test().await;

    // 1. Initially check should return false
    let req = Request::builder()
        .uri(format!(
            "/api/v1/buyer/wishlist/{}/check",
            fixture.product1_id
        ))
        .method("GET")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.buyer1_token),
        )
        .body(Body::empty())
        .unwrap();
    let resp = fixture.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let is_in: bool = serde_json::from_slice(&body).unwrap();
    assert!(!is_in);

    // 2. Add product1 to wishlist
    let req = Request::builder()
        .uri(format!("/api/v1/buyer/wishlist/{}", fixture.product1_id))
        .method("POST")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.buyer1_token),
        )
        .body(Body::empty())
        .unwrap();
    let resp = fixture.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let item: WishlistItemDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(item.buyer_id, fixture.buyer1_id);
    assert_eq!(item.product_id, fixture.product1_id);
    assert!(!item.product_name.is_empty());
    assert!(item.product_price_cents > 0);

    // 3. Check should now return true
    let req = Request::builder()
        .uri(format!(
            "/api/v1/buyer/wishlist/{}/check",
            fixture.product1_id
        ))
        .method("GET")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.buyer1_token),
        )
        .body(Body::empty())
        .unwrap();
    let resp = fixture.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let is_in: bool = serde_json::from_slice(&body).unwrap();
    assert!(is_in);

    // 4. Add product2 to wishlist
    let req = Request::builder()
        .uri(format!("/api/v1/buyer/wishlist/{}", fixture.product2_id))
        .method("POST")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.buyer1_token),
        )
        .body(Body::empty())
        .unwrap();
    let resp = fixture.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // 5. List wishlist items
    let req = Request::builder()
        .uri("/api/v1/buyer/wishlist")
        .method("GET")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.buyer1_token),
        )
        .body(Body::empty())
        .unwrap();
    let resp = fixture.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let items: Vec<WishlistItemDto> = serde_json::from_slice(&body).unwrap();
    assert_eq!(items.len(), 2);
    let product_ids: Vec<Uuid> = items.iter().map(|i| i.product_id).collect();
    assert!(product_ids.contains(&fixture.product1_id));
    assert!(product_ids.contains(&fixture.product2_id));
}

#[tokio::test]
async fn test_duplicate_wishlist_item_returns_conflict() {
    let fixture = setup_wishlist_test().await;

    // First add -> 201 Created
    let req = Request::builder()
        .uri(format!("/api/v1/buyer/wishlist/{}", fixture.product1_id))
        .method("POST")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.buyer1_token),
        )
        .body(Body::empty())
        .unwrap();
    let resp = fixture.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Second add -> 409 Conflict
    let req = Request::builder()
        .uri(format!("/api/v1/buyer/wishlist/{}", fixture.product1_id))
        .method("POST")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.buyer1_token),
        )
        .body(Body::empty())
        .unwrap();
    let resp = fixture.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);

    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let err: Value = serde_json::from_slice(&body).unwrap();
    let msg = err["error"]["message"]
        .as_str()
        .or_else(|| err["message"].as_str())
        .expect("Error message present");
    assert!(msg.contains("already in wishlist"));
}

#[tokio::test]
async fn test_remove_from_wishlist() {
    let fixture = setup_wishlist_test().await;

    // Add product1
    let req = Request::builder()
        .uri(format!("/api/v1/buyer/wishlist/{}", fixture.product1_id))
        .method("POST")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.buyer1_token),
        )
        .body(Body::empty())
        .unwrap();
    let resp = fixture.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Delete product1 -> 204 No Content
    let req = Request::builder()
        .uri(format!("/api/v1/buyer/wishlist/{}", fixture.product1_id))
        .method("DELETE")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.buyer1_token),
        )
        .body(Body::empty())
        .unwrap();
    let resp = fixture.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Delete again -> 404 Not Found
    let req = Request::builder()
        .uri(format!("/api/v1/buyer/wishlist/{}", fixture.product1_id))
        .method("DELETE")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.buyer1_token),
        )
        .body(Body::empty())
        .unwrap();
    let resp = fixture.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Check status -> false
    let req = Request::builder()
        .uri(format!(
            "/api/v1/buyer/wishlist/{}/check",
            fixture.product1_id
        ))
        .method("GET")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.buyer1_token),
        )
        .body(Body::empty())
        .unwrap();
    let resp = fixture.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let is_in: bool = serde_json::from_slice(&body).unwrap();
    assert!(!is_in);
}

#[tokio::test]
async fn test_buyer_wishlist_isolation() {
    let fixture = setup_wishlist_test().await;

    // Buyer 1 adds product1
    let req = Request::builder()
        .uri(format!("/api/v1/buyer/wishlist/{}", fixture.product1_id))
        .method("POST")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.buyer1_token),
        )
        .body(Body::empty())
        .unwrap();
    let resp = fixture.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Buyer 2 checks product1 -> false
    let req = Request::builder()
        .uri(format!(
            "/api/v1/buyer/wishlist/{}/check",
            fixture.product1_id
        ))
        .method("GET")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.buyer2_token),
        )
        .body(Body::empty())
        .unwrap();
    let resp = fixture.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let is_in: bool = serde_json::from_slice(&body).unwrap();
    assert!(!is_in);

    // Buyer 2 list -> empty
    let req = Request::builder()
        .uri("/api/v1/buyer/wishlist")
        .method("GET")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.buyer2_token),
        )
        .body(Body::empty())
        .unwrap();
    let resp = fixture.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let items: Vec<WishlistItemDto> = serde_json::from_slice(&body).unwrap();
    assert!(items.is_empty());

    // Buyer 2 adds product1 -> 201 Created (allowed for different buyer)
    let req = Request::builder()
        .uri(format!("/api/v1/buyer/wishlist/{}", fixture.product1_id))
        .method("POST")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.buyer2_token),
        )
        .body(Body::empty())
        .unwrap();
    let resp = fixture.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Buyer 2 list -> 1 item
    let req = Request::builder()
        .uri("/api/v1/buyer/wishlist")
        .method("GET")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.buyer2_token),
        )
        .body(Body::empty())
        .unwrap();
    let resp = fixture.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let items: Vec<WishlistItemDto> = serde_json::from_slice(&body).unwrap();
    assert_eq!(items.len(), 1);
}

#[tokio::test]
async fn test_add_nonexistent_product_to_wishlist() {
    let fixture = setup_wishlist_test().await;
    let non_existent_product = Uuid::new_v4();

    let req = Request::builder()
        .uri(format!("/api/v1/buyer/wishlist/{}", non_existent_product))
        .method("POST")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.buyer1_token),
        )
        .body(Body::empty())
        .unwrap();
    let resp = fixture.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
