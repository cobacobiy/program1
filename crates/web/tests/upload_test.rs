use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use jsonwebtoken::{encode, EncodingKey, Header};
use program1_contracts::JwtClaims;
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

    let state = AppState {
        store_name: "Test Store".to_string(),
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
        backup_contract: Arc::new(program1_core::backup::BackupService::new(pool.clone(), "./target/test_backups")),
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

fn create_multipart_body(
    boundary: &str,
    field_name: &str,
    filename: &str,
    content_type: &str,
    file_bytes: &[u8],
) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
            field_name, filename
        )
        .as_bytes(),
    );
    body.extend_from_slice(format!("Content-Type: {}\r\n\r\n", content_type).as_bytes());
    body.extend_from_slice(file_bytes);
    body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());
    body
}

#[tokio::test]
async fn test_upload_valid_jpeg_returns_200_and_serves_static() {
    let (app, seller_token, _) = setup_test_app().await;

    // JPEG magic bytes: FF D8 FF followed by JFIF marker
    let jpeg_bytes = vec![
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x01, 0x00,
        0x60, 0x00, 0x60, 0x00, 0x00,
    ];

    let boundary = "---------------------------testboundary123";
    let body = create_multipart_body(boundary, "image", "sample.jpg", "image/jpeg", &jpeg_bytes);

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/uploads/images")
        .header(header::AUTHORIZATION, format!("Bearer {}", seller_token))
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(body))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();

    assert!(json["url"].as_str().unwrap().starts_with("/uploads/"));
    assert!(json["filename"].as_str().unwrap().ends_with(".jpg"));
    assert_eq!(
        json["size_bytes"].as_u64().unwrap(),
        jpeg_bytes.len() as u64
    );

    let file_url = json["url"].as_str().unwrap();

    // Verify static serve via GET /uploads/{filename}
    let get_req = Request::builder()
        .method("GET")
        .uri(file_url)
        .body(Body::empty())
        .unwrap();

    let get_res = app.oneshot(get_req).await.unwrap();
    assert_eq!(get_res.status(), StatusCode::OK);
    let served_bytes = to_bytes(get_res.into_body(), usize::MAX).await.unwrap();
    assert_eq!(served_bytes.as_ref(), &jpeg_bytes[..]);
}

#[tokio::test]
async fn test_upload_valid_png_and_webp() {
    let (app, seller_token, _) = setup_test_app().await;

    // 1. PNG magic bytes: 89 50 4E 47 0D 0A 1A 0A
    let png_bytes = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52,
    ];
    let boundary_png = "---------------------------boundarypng";
    let body_png =
        create_multipart_body(boundary_png, "image", "logo.png", "image/png", &png_bytes);

    let req_png = Request::builder()
        .method("POST")
        .uri("/api/v1/uploads/images")
        .header(header::AUTHORIZATION, format!("Bearer {}", seller_token))
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary_png),
        )
        .body(Body::from(body_png))
        .unwrap();

    let res_png = app.clone().oneshot(req_png).await.unwrap();
    assert_eq!(res_png.status(), StatusCode::OK);
    let bytes_png = to_bytes(res_png.into_body(), usize::MAX).await.unwrap();
    let json_png: Value = serde_json::from_slice(&bytes_png).unwrap();
    assert!(json_png["filename"].as_str().unwrap().ends_with(".png"));

    // 2. WebP magic bytes: RIFF....WEBP
    let mut webp_bytes = Vec::new();
    webp_bytes.extend_from_slice(b"RIFF\x20\x00\x00\x00WEBPVP8 ");
    webp_bytes.extend_from_slice(&[0x00; 16]);

    let boundary_webp = "---------------------------boundarywebp";
    let body_webp = create_multipart_body(
        boundary_webp,
        "image",
        "photo.webp",
        "image/webp",
        &webp_bytes,
    );

    let req_webp = Request::builder()
        .method("POST")
        .uri("/api/v1/uploads/images")
        .header(header::AUTHORIZATION, format!("Bearer {}", seller_token))
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary_webp),
        )
        .body(Body::from(body_webp))
        .unwrap();

    let res_webp = app.oneshot(req_webp).await.unwrap();
    assert_eq!(res_webp.status(), StatusCode::OK);
    let bytes_webp = to_bytes(res_webp.into_body(), usize::MAX).await.unwrap();
    let json_webp: Value = serde_json::from_slice(&bytes_webp).unwrap();
    assert!(json_webp["filename"].as_str().unwrap().ends_with(".webp"));
}

#[tokio::test]
async fn test_upload_non_image_fails_with_validation_error() {
    let (app, seller_token, _) = setup_test_app().await;

    let text_bytes = b"Hello, this is a plain text file pretending to be an image.jpg";
    let boundary = "---------------------------boundarytxt";
    let body = create_multipart_body(boundary, "image", "malicious.jpg", "image/jpeg", text_bytes);

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/uploads/images")
        .header(header::AUTHORIZATION, format!("Bearer {}", seller_token))
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(body))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["error"]["code"], "VALIDATION_FAILED");
}

#[tokio::test]
async fn test_upload_over_5mb_fails_with_payload_too_large() {
    let (app, seller_token, _) = setup_test_app().await;

    // Create a 5.2 MB payload with valid JPEG magic bytes
    let mut large_bytes = vec![0xFF, 0xD8, 0xFF, 0xE0];
    large_bytes.resize(5 * 1024 * 1024 + 200 * 1024, 0xAA);

    let boundary = "---------------------------boundarylarge";
    let body = create_multipart_body(boundary, "image", "huge.jpg", "image/jpeg", &large_bytes);

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/uploads/images")
        .header(header::AUTHORIZATION, format!("Bearer {}", seller_token))
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(body))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn test_upload_without_auth_returns_401() {
    let (app, _, _) = setup_test_app().await;

    let jpeg_bytes = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
    let boundary = "---------------------------boundarynoauth";
    let body = create_multipart_body(boundary, "image", "sample.jpg", "image/jpeg", &jpeg_bytes);

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/uploads/images")
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(body))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_upload_with_buyer_auth_returns_403() {
    let (app, _, buyer_token) = setup_test_app().await;

    let jpeg_bytes = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
    let boundary = "---------------------------boundarybuyer";
    let body = create_multipart_body(boundary, "image", "sample.jpg", "image/jpeg", &jpeg_bytes);

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/uploads/images")
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(body))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}
