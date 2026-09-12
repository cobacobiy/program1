use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use jsonwebtoken::{encode, EncodingKey, Header};
use program1_contracts::{BackupFileDto, DatabaseHealthDto, JwtClaims};
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

async fn setup_test_app() -> (axum::Router, String, String) {
    let secret = "test-jwt-secret-key-minimum-32-characters-length!".to_string();
    let db_file = format!("./target/test_backup_app_{}.db", Uuid::new_v4());
    let pool = init_database(&format!("sqlite:{}", db_file))
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

    let temp_backup_dir = format!("./target/test_backups_{}", Uuid::new_v4());
    let backup_service = Arc::new(program1_core::backup::BackupService::new(
        pool.clone(),
        temp_backup_dir,
    ));

    let state = AppState {
        store_name: "Backup Test Store".to_string(),
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
        backup_contract: backup_service,
        rate_limiter: Arc::new(program1_web::rate_limit::IpRateLimiter::new()),
        started_at: std::time::Instant::now(),
        google_client_id: "test".to_string(),
    };

    let app = create_app(state);

    let admin_claims = JwtClaims {
        sub: Uuid::new_v4(),
        username: "superadmin".to_string(),
        role: "Super Admin".to_string(),
        accessible_menus: vec!["all".to_string()],
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp(),
        iat: chrono::Utc::now().timestamp(),
        user_type: "seller_staff".to_string(),
    };

    let staff_claims = JwtClaims {
        sub: Uuid::new_v4(),
        username: "regular_staff".to_string(),
        role: "Staff".to_string(),
        accessible_menus: vec!["orders".to_string()],
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp(),
        iat: chrono::Utc::now().timestamp(),
        user_type: "seller_staff".to_string(),
    };

    let admin_token = encode(
        &Header::default(),
        &admin_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap();

    let staff_token = encode(
        &Header::default(),
        &staff_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap();

    (app, admin_token, staff_token)
}

#[tokio::test]
async fn test_backup_unauthorized_and_forbidden() {
    let (app, _admin_token, staff_token) = setup_test_app().await;

    // 1. Without token -> 401 Unauthorized
    let req = Request::builder()
        .uri("/api/v1/admin/database/backup")
        .method("POST")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 2. With regular staff token (non-admin) -> 403 Forbidden
    let req = Request::builder()
        .uri("/api/v1/admin/database/backup")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", staff_token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_backup_lifecycle_and_health() {
    let (app, admin_token, _staff_token) = setup_test_app().await;

    // 1. Check health
    let req = Request::builder()
        .uri("/api/v1/admin/database/health")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let health: DatabaseHealthDto = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(health.status, "healthy");
    assert_eq!(health.engine, "sqlite");
    assert_eq!(health.integrity, "ok");

    // 2. Create backup
    let req = Request::builder()
        .uri("/api/v1/admin/database/backup")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let body_bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    assert_eq!(status, StatusCode::OK, "Failed creating backup: {}", String::from_utf8_lossy(&body_bytes));
    let created: BackupFileDto = serde_json::from_slice(&body_bytes).unwrap();
    assert!(created.filename.starts_with("backup_"));
    assert!(created.filename.ends_with(".db"));
    assert!(created.size_bytes > 0);

    // 3. List backups
    let req = Request::builder()
        .uri("/api/v1/admin/database/backups")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let backups: Vec<BackupFileDto> = serde_json::from_slice(&body_bytes).unwrap();
    assert!(!backups.is_empty());
    assert!(backups.iter().any(|b| b.filename == created.filename));

    // 4. Download backup
    let req = Request::builder()
        .uri(format!(
            "/api/v1/admin/database/backups/{}/download",
            created.filename
        ))
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/octet-stream"
    );
    let body_bytes = to_bytes(resp.into_body(), 10 * 1024 * 1024).await.unwrap();
    assert_eq!(body_bytes.len() as u64, created.size_bytes);
}
