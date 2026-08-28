use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use program1_core::init_database;
use program1_module_analytics::AnalyticsModule;
use program1_module_auth::AuthModule;
use program1_module_catalog::CatalogModule;
use program1_module_channel::ChannelSyncModule;
use program1_module_inventory::InventoryModule;
use program1_module_order::OrderModule;
use program1_module_user::UserModule;
use program1_web::{create_app, rate_limit::IpRateLimiter, AppState};
use serde_json::Value;
use tower::ServiceExt;

async fn setup_test_app() -> (axum::Router, Arc<IpRateLimiter>) {
    let secret = "test-jwt-secret-key-minimum-32-characters-length!".to_string();
    let pool = init_database("sqlite::memory:").await.expect("Test DB init failed");

    let user_module = Arc::new(UserModule::new(pool.clone()));
    let auth_module = Arc::new(AuthModule::new(secret, 24));
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
    let rate_limiter = Arc::new(IpRateLimiter::new());

    let _ = user_module.seed_default_users().await;
    let _ = catalog_module.seed_default_catalog().await;
    let _ = channel_module.seed_default_channels().await;

    let state = AppState {
        store_name: "Test Store".to_string(),
        store_currency: "IDR".to_string(),
        user_contract: user_module,
        auth_contract: auth_module,
        catalog_contract: catalog_module,
        inventory_contract: inventory_module,
        channel_contract: channel_module,
        order_contract: order_module,
        analytics_contract: analytics_module,
        audit_contract: audit_module,
        rate_limiter: rate_limiter.clone(),
    };


    (create_app(state), rate_limiter)
}

#[tokio::test]
async fn test_login_rate_limit_exceeded_returns_429() {
    let (app, _) = setup_test_app().await;

    let login_payload = serde_json::json!({
        "username": "admin",
        "password": "wrongpassword123"
    });

    // Make 5 requests within the 5 req/min limit
    for _ in 0..5 {
        let req = Request::builder()
            .uri("/api/v1/auth/login")
            .method("POST")
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-forwarded-for", "192.168.1.100")
            .body(Body::from(serde_json::to_vec(&login_payload).unwrap()))
            .unwrap();

        let response = app.clone().oneshot(req).await.unwrap();
        // Requests should be processed by the handler (returns 400 bad request due to wrong password, not 429)
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(response.headers().get("x-ratelimit-limit").is_some());
    }

    // 6th request must exceed limit and return 429 Too Many Requests
    let req = Request::builder()
        .uri("/api/v1/auth/login")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-forwarded-for", "192.168.1.100")
        .body(Body::from(serde_json::to_vec(&login_payload).unwrap()))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

    let headers = response.headers();
    let retry_after_hdr: u64 = headers.get(header::RETRY_AFTER).unwrap().to_str().unwrap().parse().unwrap();
    assert!(retry_after_hdr > 0 && retry_after_hdr <= 60);
    assert_eq!(headers.get("x-ratelimit-limit").unwrap().to_str().unwrap(), "5");
    assert_eq!(headers.get("x-ratelimit-remaining").unwrap().to_str().unwrap(), "0");

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"], "rate_limit_exceeded");
    let retry_val = body["retry_after"].as_u64().unwrap();
    assert!(retry_val > 0 && retry_val <= 60);
}


#[tokio::test]
async fn test_forwarded_for_ip_isolation() {
    let (app, _) = setup_test_app().await;

    let login_payload = serde_json::json!({
        "username": "admin",
        "password": "wrongpassword123"
    });

    // Exhaust quota for IP 10.0.0.1
    for _ in 0..5 {
        let req = Request::builder()
            .uri("/api/v1/auth/login")
            .method("POST")
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-forwarded-for", "10.0.0.1")
            .body(Body::from(serde_json::to_vec(&login_payload).unwrap()))
            .unwrap();

        let _ = app.clone().oneshot(req).await.unwrap();
    }

    // IP 10.0.0.1 is blocked
    let req_blocked = Request::builder()
        .uri("/api/v1/auth/login")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-forwarded-for", "10.0.0.1")
        .body(Body::from(serde_json::to_vec(&login_payload).unwrap()))
        .unwrap();
    let res_blocked = app.clone().oneshot(req_blocked).await.unwrap();
    assert_eq!(res_blocked.status(), StatusCode::TOO_MANY_REQUESTS);

    // Different IP 10.0.0.2 should still succeed
    let req_allowed = Request::builder()
        .uri("/api/v1/auth/login")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-forwarded-for", "10.0.0.2")
        .body(Body::from(serde_json::to_vec(&login_payload).unwrap()))
        .unwrap();
    let res_allowed = app.oneshot(req_allowed).await.unwrap();
    assert_eq!(res_allowed.status(), StatusCode::BAD_REQUEST); // handled, not 429
}

#[tokio::test]
async fn test_health_check_is_unlimited() {
    let (app, _) = setup_test_app().await;

    for _ in 0..15 {
        let req = Request::builder()
            .uri("/health")
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
