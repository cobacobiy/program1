use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use program1_contracts::{
    JwtClaims, PaginatedResponse, ProductRatingSummaryDto, ProductReviewDto, PublicReviewDto,
};
use program1_core::{database::DbPool, init_database};
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
use serde_json::{json, Value};
use sqlx::Row;
use tower::ServiceExt;
use uuid::Uuid;

struct TestFixture {
    app: axum::Router,
    pool: DbPool,
    seller_token: String,
    staff_token: String,
    buyer1_token: String,
    buyer1_id: Uuid,
    buyer2_token: String,
    _buyer2_id: Uuid,
    product1_id: Uuid,
    product2_id: Uuid,
    delivered_order_id: Uuid,
    pending_order_id: Uuid,
}

async fn setup_review_test() -> TestFixture {
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
        store_name: "Review Test Store".to_string(),
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

    // Seller Admin token
    let seller_claims = JwtClaims {
        sub: Uuid::new_v4(),
        username: "admin_seller".to_string(),
        role: "Super Admin".to_string(),
        accessible_menus: vec!["all".to_string()],
        exp: (Utc::now() + Duration::hours(1)).timestamp(),
        iat: Utc::now().timestamp(),
        user_type: "seller_staff".to_string(),
    };
    let seller_token = encode(
        &Header::default(),
        &seller_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap();

    // Non-admin seller staff token (role: "Staff")
    let staff_claims = JwtClaims {
        sub: Uuid::new_v4(),
        username: "staff_operator".to_string(),
        role: "Staff".to_string(),
        accessible_menus: vec!["orders".to_string()],
        exp: (Utc::now() + Duration::hours(1)).timestamp(),
        iat: Utc::now().timestamp(),
        user_type: "seller_staff".to_string(),
    };
    let staff_token = encode(
        &Header::default(),
        &staff_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap();

    // Buyer 1
    let buyer1_id = Uuid::new_v4();
    let now_str = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO buyer_accounts (id, email, full_name, phone_number, phone_verified, is_active, created_at, updated_at)
         VALUES ($1, 'buyer1@test.com', 'Buyer Satu', '+62811111111', 1, 1, $2, $3)"
    )
    .bind(buyer1_id.to_string())
    .bind(&now_str)
    .bind(&now_str)
    .execute(&pool)
    .await
    .unwrap();

    let buyer1_claims = JwtClaims {
        sub: buyer1_id,
        username: "buyer1@test.com".to_string(),
        role: "buyer".to_string(),
        accessible_menus: vec![],
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

    // Buyer 2
    let buyer2_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO buyer_accounts (id, email, full_name, phone_number, phone_verified, is_active, created_at, updated_at)
         VALUES ($1, 'buyer2@test.com', 'Buyer Dua', '+62822222222', 1, 1, $2, $3)"
    )
    .bind(buyer2_id.to_string())
    .bind(&now_str)
    .bind(&now_str)
    .execute(&pool)
    .await
    .unwrap();

    let buyer2_claims = JwtClaims {
        sub: buyer2_id,
        username: "buyer2@test.com".to_string(),
        role: "buyer".to_string(),
        accessible_menus: vec![],
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

    // Insert Product 1 & Product 2
    let product1_id = Uuid::new_v4();
    let product2_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO catalog_items (id, name, sku, category, price, stock, description)
         VALUES ($1, 'Product Satu', 'SKU-PROD-01', 'General', 150000.0, 50, 'Desc 1')",
    )
    .bind(product1_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO catalog_items (id, name, sku, category, price, stock, description)
         VALUES ($1, 'Product Dua', 'SKU-PROD-02', 'General', 250000.0, 50, 'Desc 2')",
    )
    .bind(product2_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    // Delivered Order for Buyer 1 (contains only Product 1)
    let delivered_order_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO orders (id, customer_name, customer_email, shipping_address, total_amount, status, buyer_id)
         VALUES ($1, 'Buyer Satu', 'buyer1@test.com', 'Jakarta', 150000.0, 'delivered', $2)"
    )
    .bind(delivered_order_id.to_string())
    .bind(buyer1_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO order_items (id, order_id, product_id, product_name, quantity, unit_price, total_price)
         VALUES ($1, $2, $3, 'Product Satu', 1, 150000.0, 150000.0)"
    )
    .bind(Uuid::new_v4().to_string())
    .bind(delivered_order_id.to_string())
    .bind(product1_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    // Pending Order for Buyer 1
    let pending_order_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO orders (id, customer_name, customer_email, shipping_address, total_amount, status, buyer_id)
         VALUES ($1, 'Buyer Satu', 'buyer1@test.com', 'Jakarta', 150000.0, 'pending', $2)"
    )
    .bind(pending_order_id.to_string())
    .bind(buyer1_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO order_items (id, order_id, product_id, product_name, quantity, unit_price, total_price)
         VALUES ($1, $2, $3, 'Product Satu', 1, 150000.0, 150000.0)"
    )
    .bind(Uuid::new_v4().to_string())
    .bind(pending_order_id.to_string())
    .bind(product1_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    TestFixture {
        app,
        pool: pool.clone(),
        seller_token,
        staff_token,
        buyer1_token,
        buyer1_id,
        buyer2_token,
        _buyer2_id: buyer2_id,
        product1_id,
        product2_id,
        delivered_order_id,
        pending_order_id,
    }
}

#[tokio::test]
async fn test_create_review_success_and_public_views() {
    let fixture = setup_review_test().await;

    // 1. Submit review for delivered product
    let create_payload = json!({
        "product_id": fixture.product1_id,
        "order_id": fixture.delivered_order_id,
        "rating": 5,
        "review_text": "Luar biasa, pengiriman cepat & barang original! <b>Best!</b>"
    });

    let res = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/buyer/reviews")
                .header(header::CONTENT_TYPE, "application/json")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", fixture.buyer1_token),
                )
                .body(Body::from(create_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
    let body_bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let review: ProductReviewDto = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(review.rating, 5);
    assert_eq!(review.buyer_id, fixture.buyer1_id);
    assert_eq!(review.buyer_name, "Buyer Satu");
    assert!(review.is_visible);
    assert_eq!(
        review.review_text.as_deref(),
        Some("Luar biasa, pengiriman cepat & barang original! Best!")
    );

    // Verify PII is NOT leaked in JSON response
    let raw_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(raw_json.get("email").is_none());
    assert!(raw_json.get("phone").is_none());
    assert!(raw_json.get("phone_number").is_none());

    // 2. Public GET /api/v1/catalog/:id/reviews
    let pub_res = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/catalog/{}/reviews", fixture.product1_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(pub_res.status(), StatusCode::OK);
    let pub_bytes = to_bytes(pub_res.into_body(), usize::MAX).await.unwrap();
    let paginated: PaginatedResponse<PublicReviewDto> = serde_json::from_slice(&pub_bytes).unwrap();
    assert_eq!(paginated.total, 1);
    assert_eq!(paginated.data.len(), 1);
    assert_eq!(paginated.data[0].buyer_name, "Buyer Satu");

    // 3. Public GET /api/v1/catalog/:id/rating
    let rating_res = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/catalog/{}/rating", fixture.product1_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(rating_res.status(), StatusCode::OK);
    let rating_bytes = to_bytes(rating_res.into_body(), usize::MAX).await.unwrap();
    let summary: ProductRatingSummaryDto = serde_json::from_slice(&rating_bytes).unwrap();
    assert_eq!(summary.total_reviews, 1);
    assert_eq!(summary.average_rating, 5.0);
    assert_eq!(summary.rating_distribution, [0, 0, 0, 0, 1]);
}

#[tokio::test]
async fn test_negative_security_reviews() {
    let fixture = setup_review_test().await;

    // 1. Unauthenticated review creation -> 401
    let unauth_res = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/buyer/reviews")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "product_id": fixture.product1_id,
                        "order_id": fixture.delivered_order_id,
                        "rating": 5
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauth_res.status(), StatusCode::UNAUTHORIZED);

    // 2. Staff token trying to hit buyer review creation -> 403
    let staff_res = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/buyer/reviews")
                .header(header::CONTENT_TYPE, "application/json")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", fixture.seller_token),
                )
                .body(Body::from(
                    json!({
                        "product_id": fixture.product1_id,
                        "order_id": fixture.delivered_order_id,
                        "rating": 5
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(staff_res.status(), StatusCode::FORBIDDEN);

    // 3. Review non-delivered order -> 400
    let pending_res = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/buyer/reviews")
                .header(header::CONTENT_TYPE, "application/json")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", fixture.buyer1_token),
                )
                .body(Body::from(
                    json!({
                        "product_id": fixture.product1_id,
                        "order_id": fixture.pending_order_id,
                        "rating": 4
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(pending_res.status(), StatusCode::BAD_REQUEST);

    // 4. Cross-buyer IDOR: Buyer 2 tries to review Buyer 1's delivered order -> 400 (or 403)
    let idor_res = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/buyer/reviews")
                .header(header::CONTENT_TYPE, "application/json")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", fixture.buyer2_token),
                )
                .body(Body::from(
                    json!({
                        "product_id": fixture.product1_id,
                        "order_id": fixture.delivered_order_id,
                        "rating": 5
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(idor_res.status(), StatusCode::BAD_REQUEST);

    // 5. Review product not contained in the order -> 400
    let absent_prod_res = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/buyer/reviews")
                .header(header::CONTENT_TYPE, "application/json")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", fixture.buyer1_token),
                )
                .body(Body::from(
                    json!({
                        "product_id": fixture.product2_id,
                        "order_id": fixture.delivered_order_id,
                        "rating": 5
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(absent_prod_res.status(), StatusCode::BAD_REQUEST);

    // 6. Rating out of bounds (0 or 6) -> 422 or 400 validation error
    let invalid_rating_res = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/buyer/reviews")
                .header(header::CONTENT_TYPE, "application/json")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", fixture.buyer1_token),
                )
                .body(Body::from(
                    json!({
                        "product_id": fixture.product1_id,
                        "order_id": fixture.delivered_order_id,
                        "rating": 6
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        invalid_rating_res.status() == StatusCode::BAD_REQUEST
            || invalid_rating_res.status() == StatusCode::UNPROCESSABLE_ENTITY
    );

    // 7. Duplicate review -> exactly one review per buyer/product/order
    let valid_review_res = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/buyer/reviews")
                .header(header::CONTENT_TYPE, "application/json")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", fixture.buyer1_token),
                )
                .body(Body::from(
                    json!({
                        "product_id": fixture.product1_id,
                        "order_id": fixture.delivered_order_id,
                        "rating": 4
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(valid_review_res.status(), StatusCode::CREATED);

    let dup_res = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/buyer/reviews")
                .header(header::CONTENT_TYPE, "application/json")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", fixture.buyer1_token),
                )
                .body(Body::from(
                    json!({
                        "product_id": fixture.product1_id,
                        "order_id": fixture.delivered_order_id,
                        "rating": 5
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(dup_res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_admin_review_moderation_and_hidden_leakage() {
    let fixture = setup_review_test().await;

    // Create review
    let create_res = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/buyer/reviews")
                .header(header::CONTENT_TYPE, "application/json")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", fixture.buyer1_token),
                )
                .body(Body::from(
                    json!({
                        "product_id": fixture.product1_id,
                        "order_id": fixture.delivered_order_id,
                        "rating": 2,
                        "review_text": "Kurang puas dengan kemasan."
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_res.status(), StatusCode::CREATED);
    let create_bytes = to_bytes(create_res.into_body(), usize::MAX).await.unwrap();
    let review: ProductReviewDto = serde_json::from_slice(&create_bytes).unwrap();

    // Admin lists reviews
    let admin_list_res = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/reviews")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", fixture.seller_token),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(admin_list_res.status(), StatusCode::OK);
    let admin_bytes = to_bytes(admin_list_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let admin_page: PaginatedResponse<ProductReviewDto> =
        serde_json::from_slice(&admin_bytes).unwrap();
    assert_eq!(admin_page.total, 1);

    // Admin moderates review: hide it
    let mod_res = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/admin/reviews/{}/visibility", review.id))
                .header(header::CONTENT_TYPE, "application/json")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", fixture.seller_token),
                )
                .body(Body::from(json!({ "is_visible": false }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(mod_res.status(), StatusCode::OK);
    let mod_bytes = to_bytes(mod_res.into_body(), usize::MAX).await.unwrap();
    let moderated: ProductReviewDto = serde_json::from_slice(&mod_bytes).unwrap();
    assert!(!moderated.is_visible);

    // Verify hidden review is NOT visible to public
    let pub_reviews_res = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/catalog/{}/reviews", fixture.product1_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let pub_bytes = to_bytes(pub_reviews_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let pub_page: PaginatedResponse<PublicReviewDto> = serde_json::from_slice(&pub_bytes).unwrap();
    assert_eq!(pub_page.total, 0);
    assert_eq!(pub_page.data.len(), 0);

    // Verify rating summary does NOT count hidden review
    let rating_res = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/catalog/{}/rating", fixture.product1_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let rating_bytes = to_bytes(rating_res.into_body(), usize::MAX).await.unwrap();
    let summary: ProductRatingSummaryDto = serde_json::from_slice(&rating_bytes).unwrap();
    assert_eq!(summary.total_reviews, 0);
    assert_eq!(summary.average_rating, 0.0);
    assert_eq!(summary.rating_distribution, [0, 0, 0, 0, 0]);

    // Non-admin cannot moderate -> 401
    let unauth_mod = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/admin/reviews/{}/visibility", review.id))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "is_visible": true }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauth_mod.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_non_admin_staff_cannot_access_or_moderate_reviews() {
    let fixture = setup_review_test().await;

    // 1. Non-admin seller staff tries GET /api/v1/admin/reviews -> must get 403 Forbidden
    let list_req = Request::builder()
        .method("GET")
        .uri("/api/v1/admin/reviews")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.staff_token),
        )
        .body(Body::empty())
        .unwrap();
    let list_res = fixture.app.clone().oneshot(list_req).await.unwrap();
    assert_eq!(list_res.status(), StatusCode::FORBIDDEN);

    // 2. Non-admin seller staff tries PATCH /api/v1/admin/reviews/{id}/visibility -> must get 403 Forbidden
    let mod_req = Request::builder()
        .method("PATCH")
        .uri(format!(
            "/api/v1/admin/reviews/{}/visibility",
            Uuid::new_v4()
        ))
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.staff_token),
        )
        .body(Body::from(json!({ "is_visible": false }).to_string()))
        .unwrap();
    let mod_res = fixture.app.clone().oneshot(mod_req).await.unwrap();
    assert_eq!(mod_res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_public_review_endpoint_omits_internal_ids_and_pii() {
    let fixture = setup_review_test().await;

    // 1. Create a review via buyer
    let create_payload = json!({
        "product_id": fixture.product1_id,
        "order_id": fixture.delivered_order_id,
        "rating": 5,
        "review_text": "Privasi terjaga, produk bagus!"
    });
    let create_res = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/buyer/reviews")
                .header(header::CONTENT_TYPE, "application/json")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", fixture.buyer1_token),
                )
                .body(Body::from(create_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_res.status(), StatusCode::CREATED);

    // 2. Query public endpoint without ANY authentication header
    let pub_res = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/catalog/{}/reviews", fixture.product1_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(pub_res.status(), StatusCode::OK);

    let bytes = to_bytes(pub_res.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(bytes.to_vec()).unwrap();
    let json_val: Value = serde_json::from_str(&body_str).unwrap();

    let items = json_val["data"].as_array().expect("data array");
    assert_eq!(items.len(), 1);
    let item = &items[0];

    // Allowed public fields must exist
    assert!(item.get("id").is_some());
    assert_eq!(item["product_id"], fixture.product1_id.to_string());
    assert_eq!(item["buyer_name"], "Buyer Satu");
    assert_eq!(item["rating"], 5);
    assert_eq!(item["review_text"], "Privasi terjaga, produk bagus!");
    assert!(item.get("created_at").is_some());
    assert!(item.get("updated_at").is_some());

    // Internal identifiers and PII MUST NOT exist
    assert!(item.get("buyer_id").is_none(), "buyer_id must be omitted");
    assert!(item.get("order_id").is_none(), "order_id must be omitted");
    assert!(
        item.get("is_visible").is_none(),
        "is_visible must be omitted"
    );
    assert!(item.get("email").is_none(), "email must be omitted");
    assert!(item.get("phone").is_none(), "phone must be omitted");
    assert!(
        item.get("phone_number").is_none(),
        "phone_number must be omitted"
    );

    // Entire response body must not leak buyer_id or order_id
    assert!(
        !body_str.contains(&fixture.buyer1_id.to_string()),
        "buyer_id UUID leaked in public response"
    );
    assert!(
        !body_str.contains(&fixture.delivered_order_id.to_string()),
        "order_id UUID leaked in public response"
    );
}

#[tokio::test]
async fn test_admin_reviews_true_pagination_beyond_50_items() {
    let fixture = setup_review_test().await;

    // Seed 65 reviews directly into database to test >50 records boundaries
    let now_str = Utc::now().to_rfc3339();
    for i in 1..=65 {
        let rev_id = Uuid::new_v4().to_string();
        let ord_id = Uuid::new_v4().to_string();
        let rating = (i % 5) + 1;

        sqlx::query(
            "INSERT INTO orders (id, customer_name, customer_email, shipping_address, total_amount, status, buyer_id)
             VALUES ($1, 'Buyer Bulk', 'bulk@test.com', 'Jakarta', 100000.0, 'delivered', $2)"
        )
        .bind(&ord_id)
        .bind(fixture.buyer1_id.to_string())
        .execute(&fixture.pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO product_reviews (id, product_id, buyer_id, order_id, rating, review_text, is_visible, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, 1, $7, $7)"
        )
        .bind(&rev_id)
        .bind(fixture.product1_id.to_string())
        .bind(fixture.buyer1_id.to_string())
        .bind(&ord_id)
        .bind(rating)
        .bind(format!("Bulk review #{}", i))
        .bind(&now_str)
        .execute(&fixture.pool)
        .await
        .unwrap();
    }

    // 1. Page 1 with page_size=20 -> total=65, total_pages=4, returns 20 items
    let req_p1 = Request::builder()
        .method("GET")
        .uri("/api/v1/admin/reviews?page=1&page_size=20")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.seller_token),
        )
        .body(Body::empty())
        .unwrap();
    let res_p1 = fixture.app.clone().oneshot(req_p1).await.unwrap();
    assert_eq!(res_p1.status(), StatusCode::OK);
    let bytes_p1 = to_bytes(res_p1.into_body(), usize::MAX).await.unwrap();
    let page1: PaginatedResponse<ProductReviewDto> = serde_json::from_slice(&bytes_p1).unwrap();
    assert_eq!(page1.total, 65);
    assert_eq!(page1.total_pages, 4);
    assert_eq!(page1.page, 1);
    assert_eq!(page1.page_size, 20);
    assert_eq!(page1.data.len(), 20);

    // 2. Page 4 with page_size=20 -> returns remaining 5 items
    let req_p4 = Request::builder()
        .method("GET")
        .uri("/api/v1/admin/reviews?page=4&page_size=20")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.seller_token),
        )
        .body(Body::empty())
        .unwrap();
    let res_p4 = fixture.app.clone().oneshot(req_p4).await.unwrap();
    assert_eq!(res_p4.status(), StatusCode::OK);
    let bytes_p4 = to_bytes(res_p4.into_body(), usize::MAX).await.unwrap();
    let page4: PaginatedResponse<ProductReviewDto> = serde_json::from_slice(&bytes_p4).unwrap();
    assert_eq!(page4.total, 65);
    assert_eq!(page4.total_pages, 4);
    assert_eq!(page4.page, 4);
    assert_eq!(page4.page_size, 20);
    assert_eq!(page4.data.len(), 5);

    // 3. Page 2 with page_size=50 (beyond 50 boundary) -> returns 15 items
    let req_p2_50 = Request::builder()
        .method("GET")
        .uri("/api/v1/admin/reviews?page=2&page_size=50")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.seller_token),
        )
        .body(Body::empty())
        .unwrap();
    let res_p2_50 = fixture.app.clone().oneshot(req_p2_50).await.unwrap();
    assert_eq!(res_p2_50.status(), StatusCode::OK);
    let bytes_p2_50 = to_bytes(res_p2_50.into_body(), usize::MAX).await.unwrap();
    let page2_50: PaginatedResponse<ProductReviewDto> =
        serde_json::from_slice(&bytes_p2_50).unwrap();
    assert_eq!(page2_50.total, 65);
    assert_eq!(page2_50.total_pages, 2);
    assert_eq!(page2_50.page, 2);
    assert_eq!(page2_50.page_size, 50);
    assert_eq!(page2_50.data.len(), 15);
}

#[tokio::test]
async fn test_admin_moderation_atomic_audit_failure_rollback() {
    let fixture = setup_review_test().await;

    // 1. Create a review
    let create_payload = json!({
        "product_id": fixture.product1_id,
        "order_id": fixture.delivered_order_id,
        "rating": 4,
        "review_text": "Produk oke banget"
    });
    let create_res = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/buyer/reviews")
                .header(header::CONTENT_TYPE, "application/json")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", fixture.buyer1_token),
                )
                .body(Body::from(create_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_res.status(), StatusCode::CREATED);
    let create_bytes = to_bytes(create_res.into_body(), usize::MAX).await.unwrap();
    let review: ProductReviewDto = serde_json::from_slice(&create_bytes).unwrap();
    assert!(review.is_visible);

    // 2. Install trigger to simulate audit log failure
    sqlx::query(
        "CREATE TRIGGER fail_audit_insert BEFORE INSERT ON audit_logs BEGIN SELECT RAISE(ABORT, 'Simulated audit log insertion failure'); END;"
    )
    .execute(&fixture.pool)
    .await
    .unwrap();

    // 3. Admin attempts to hide review -> must fail (500) due to fail-closed atomic transaction
    let mod_req = Request::builder()
        .method("PATCH")
        .uri(format!("/api/v1/admin/reviews/{}/visibility", review.id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.seller_token),
        )
        .body(Body::from(json!({ "is_visible": false }).to_string()))
        .unwrap();
    let mod_res = fixture.app.clone().oneshot(mod_req).await.unwrap();
    assert_eq!(mod_res.status(), StatusCode::INTERNAL_SERVER_ERROR);

    // 4. Verify review visibility rolled back and is STILL visible
    let row = sqlx::query("SELECT is_visible FROM product_reviews WHERE id = $1")
        .bind(review.id.to_string())
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    let is_visible_int: i64 = row.get("is_visible");
    assert_eq!(
        is_visible_int, 1,
        "Review must remain visible upon audit failure rollback"
    );

    // 5. Drop trigger and retry -> must succeed
    sqlx::query("DROP TRIGGER fail_audit_insert")
        .execute(&fixture.pool)
        .await
        .unwrap();

    let retry_req = Request::builder()
        .method("PATCH")
        .uri(format!("/api/v1/admin/reviews/{}/visibility", review.id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", fixture.seller_token),
        )
        .body(Body::from(json!({ "is_visible": false }).to_string()))
        .unwrap();
    let retry_res = fixture.app.clone().oneshot(retry_req).await.unwrap();
    assert_eq!(retry_res.status(), StatusCode::OK);

    // 6. Verify audit log entry was created
    let audit_row = sqlx::query(
        "SELECT action, actor_username, details FROM audit_logs WHERE resource_id = $1",
    )
    .bind(review.id.to_string())
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    let action: String = audit_row.get("action");
    let actor_username: String = audit_row.get("actor_username");
    let details: String = audit_row.get("details");
    assert_eq!(action, "review.hide");
    assert_eq!(actor_username, "admin_seller");
    assert!(details.contains("\"previous_is_visible\":true"));
    assert!(details.contains("\"new_is_visible\":false"));
}
