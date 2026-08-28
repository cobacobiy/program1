use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use program1_contracts::{AuthContract, JwtClaims};
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

async fn setup_test_app() -> (axum::Router, Arc<AuthModule>, String) {
    let secret = "test-jwt-secret-key-minimum-32-characters-length!".to_string();
    let pool = init_database("sqlite::memory:").await.expect("Test DB init failed");

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

    let _ = user_module.seed_default_users().await;
    let _ = catalog_module.seed_default_catalog().await;
    let _ = channel_module.seed_default_channels().await;

    let state = AppState {
        store_name: "Test Store".to_string(),
        store_currency: "IDR".to_string(),
        user_contract: user_module,
        auth_contract: auth_module.clone(),
        catalog_contract: catalog_module,
        inventory_contract: inventory_module,
        channel_contract: channel_module,
        order_contract: order_module,
        analytics_contract: analytics_module,
        audit_contract: audit_module,
        rate_limiter: Arc::new(program1_web::rate_limit::IpRateLimiter::new()),
        started_at: std::time::Instant::now(),
    };




    let router = create_app(state);
    (router, auth_module, secret)
}

#[tokio::test]
async fn test_public_routes_accessible_without_token() {
    let (app, _, _) = setup_test_app().await;

    // 1. Health check
    let req = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 2. Store info
    let req = Request::builder()
        .uri("/api/v1/store/info")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 3. Catalog list
    let req = Request::builder()
        .uri("/api/v1/catalog")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_login_returns_valid_jwt_token() {
    let (app, auth_module, _) = setup_test_app().await;

    let login_body = serde_json::json!({
        "username": "admin",
        "password": "admin123"
    });

    let req = Request::builder()
        .uri("/api/v1/auth/login")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&login_body).unwrap()))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_json: Value = serde_json::from_slice(&body_bytes).unwrap();

    let access_token = body_json["access_token"].as_str().expect("access_token should be string");
    assert!(!access_token.is_empty());

    // Validate the token claims
    let claims = auth_module.validate_token(access_token).expect("Token should be valid");
    assert_eq!(claims.username, "admin");
    assert_eq!(claims.role, "Super Admin");
}

#[tokio::test]
async fn test_protected_routes_reject_missing_token() {
    let (app, _, _) = setup_test_app().await;

    let req = Request::builder()
        .uri("/api/v1/inventory")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body_json["error"], "authentication_required");
}

#[tokio::test]
async fn test_protected_routes_accept_valid_token() {
    let (app, auth_module, _) = setup_test_app().await;

    // Login as staff
    let staff_claims = program1_contracts::UserAccountDto {
        id: Uuid::new_v4(),
        username: "staff_gudang".to_string(),
        full_name: "Staff Gudang".to_string(),
        role: "Warehouse Manager".to_string(),
        accessible_menus: vec!["stocks".to_string()],
        is_active: true,
        created_at: Utc::now(),
    };

    let token = auth_module.generate_token(&staff_claims).unwrap();

    let req = Request::builder()
        .uri("/api/v1/inventory")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_routes_reject_non_admin_token() {
    let (app, auth_module, _) = setup_test_app().await;

    let staff_claims = program1_contracts::UserAccountDto {
        id: Uuid::new_v4(),
        username: "staff_cs".to_string(),
        full_name: "Staff CS".to_string(),
        role: "Customer Support".to_string(),
        accessible_menus: vec!["chat".to_string()],
        is_active: true,
        created_at: Utc::now(),
    };

    let token = auth_module.generate_token(&staff_claims).unwrap();

    let req = Request::builder()
        .uri("/api/v1/analytics")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body_json["error"], "insufficient_permissions");
}

#[tokio::test]
async fn test_admin_routes_accept_admin_token() {
    let (app, auth_module, _) = setup_test_app().await;

    let admin_claims = program1_contracts::UserAccountDto {
        id: Uuid::new_v4(),
        username: "admin".to_string(),
        full_name: "Admin Super".to_string(),
        role: "Super Admin".to_string(),
        accessible_menus: vec!["dashboard".to_string(), "reports".to_string()],
        is_active: true,
        created_at: Utc::now(),
    };

    let token = auth_module.generate_token(&admin_claims).unwrap();

    let req = Request::builder()
        .uri("/api/v1/analytics")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_expired_token_returns_token_expired_error() {
    let (app, _, secret) = setup_test_app().await;

    let now = Utc::now().timestamp();
    let expired_claims = JwtClaims {
        sub: Uuid::new_v4(),
        username: "admin".to_string(),
        role: "Super Admin".to_string(),
        accessible_menus: vec![],
        exp: now - 3600, // 1 hour in the past
        iat: now - 7200,
    };

    let expired_token = encode(
        &Header::default(),
        &expired_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap();

    let req = Request::builder()
        .uri("/api/v1/inventory")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", expired_token))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body_json["error"], "token_expired");
}
