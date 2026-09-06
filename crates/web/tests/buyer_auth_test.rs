use async_trait::async_trait;
use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use std::sync::Arc;

use chrono::Utc;
use program1_contracts::{AuthContract, UserAccountDto};
use program1_core::init_database;
use program1_module_analytics::AnalyticsModule;
use program1_module_audit::AuditModule;
use program1_module_auth::AuthModule;
use program1_module_buyer::{BuyerModule, GoogleClaims, GoogleTokenVerifier, SmsOtpSender};
use program1_module_catalog::CatalogModule;
use program1_module_channel::ChannelSyncModule;
use program1_module_inventory::InventoryModule;
use program1_module_order::OrderModule;
use program1_module_user::UserModule;
use program1_web::{create_app, AppState};
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tower::ServiceExt;
use uuid::Uuid;

struct TestGoogleVerifier;
#[async_trait]
impl GoogleTokenVerifier for TestGoogleVerifier {
    async fn verify(
        &self,
        id_token: &str,
    ) -> Result<GoogleClaims, program1_contracts::ContractError> {
        if id_token.starts_with("test_google:") {
            let parts: Vec<&str> = id_token.split(':').collect();
            Ok(GoogleClaims {
                sub: parts.get(1).unwrap_or(&"sub123").to_string(),
                email: parts.get(2).unwrap_or(&"test@buyer.com").to_string(),
                name: parts.get(3).unwrap_or(&"Test Buyer").to_string(),
                picture: Some("https://example.com/pic.jpg".to_string()),
            })
        } else {
            Err(program1_contracts::ContractError::ValidationError(
                "Invalid Google Token".to_string(),
            ))
        }
    }
}

struct TestSmsSender {
    otps: Arc<RwLock<HashMap<String, String>>>,
}
#[async_trait]
impl SmsOtpSender for TestSmsSender {
    async fn send_otp(
        &self,
        phone_number: &str,
        otp_code: &str,
    ) -> Result<(), program1_contracts::ContractError> {
        let mut lock = self.otps.write().await;
        lock.insert(phone_number.to_string(), otp_code.to_string());
        Ok(())
    }
}

async fn setup_buyer_test_app_with_pool() -> (
    axum::Router,
    Arc<RwLock<HashMap<String, String>>>,
    Arc<AuthModule>,
    program1_core::database::DbPool,
) {
    let secret = "test-jwt-secret-key-minimum-32-characters-length!".to_string();
    let pool = init_database("sqlite::memory:")
        .await
        .expect("Test DB init failed");

    let user_module = Arc::new(
        UserModule::new(pool.clone())
            .with_dev_support_password(Some("dev_support_secret_token_12345!".to_string())),
    );
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

    let sent_otps = Arc::new(RwLock::new(HashMap::new()));
    let sms_sender = Arc::new(TestSmsSender {
        otps: sent_otps.clone(),
    });
    let google_verifier = Arc::new(TestGoogleVerifier);

    let buyer_module = Arc::new(BuyerModule::new(
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

    let _ = user_module.seed_default_users().await;
    let _ = catalog_module.seed_default_catalog().await;
    let _ = channel_module.seed_default_channels().await;

    let state = AppState {
        store_name: "Test Store".to_string(),
        store_currency: "IDR".to_string(),
        store_whatsapp_number: "085810007735".to_string(),
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
        rate_limiter: Arc::new(program1_web::rate_limit::IpRateLimiter::new()),
        started_at: std::time::Instant::now(),
        google_client_id: "test-google-client-id.apps.googleusercontent.com".to_string(),
    };

    let router = create_app(state);
    (router, sent_otps, auth_module, pool)
}

async fn setup_buyer_test_app() -> (
    axum::Router,
    Arc<RwLock<HashMap<String, String>>>,
    Arc<AuthModule>,
) {
    let (router, sent_otps, auth_module, _) = setup_buyer_test_app_with_pool().await;
    (router, sent_otps, auth_module)
}

#[tokio::test]
async fn test_buyer_google_login_and_phone_verification_flow() {
    let (app, sent_otps, _) = setup_buyer_test_app().await;

    // 1. Google login (first time)
    let login_payload = json!({
        "id_token": "test_google:g_sub_101:citra@buyer.com:Citra Lestari"
    });
    let req = Request::builder()
        .uri("/api/v1/buyer/auth/google")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&login_payload).unwrap()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let buyer_token = json["access_token"].as_str().unwrap().to_string();
    assert!(json["requires_phone_verification"].as_bool().unwrap());
    assert_eq!(json["buyer"]["email"], "citra@buyer.com");

    // 2. Request OTP
    let otp_req_payload = json!({
        "phone_number": "+6281234567890"
    });
    let req = Request::builder()
        .uri("/api/v1/buyer/otp/request")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&otp_req_payload).unwrap()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Retrieve generated OTP from mock SMS
    let otp_code = {
        let lock = sent_otps.read().await;
        lock.get("+6281234567890")
            .cloned()
            .expect("OTP sent to mock SMS")
    };

    // 3. Verify OTP
    let verify_payload = json!({
        "phone_number": "+6281234567890",
        "code": otp_code
    });
    let req = Request::builder()
        .uri("/api/v1/buyer/otp/verify")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&verify_payload).unwrap()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let verified_json: Value = serde_json::from_slice(&body).unwrap();
    assert!(verified_json["phone_verified"].as_bool().unwrap());
    assert_eq!(verified_json["phone_number"], "+6281234567890");

    // 4. Subsequent Google login: requires_phone_verification is FALSE
    let req = Request::builder()
        .uri("/api/v1/buyer/auth/google")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&login_payload).unwrap()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json2: Value = serde_json::from_slice(&body).unwrap();
    assert!(!json2["requires_phone_verification"].as_bool().unwrap());
}

#[tokio::test]
async fn test_buyer_addresses_multiple_with_single_default() {
    let (app, _, _) = setup_buyer_test_app().await;

    // Login buyer
    let login_payload = json!({
        "id_token": "test_google:g_sub_addr:address_user@buyer.com:Addr User"
    });
    let req = Request::builder()
        .uri("/api/v1/buyer/auth/google")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&login_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let buyer_token = json["access_token"].as_str().unwrap().to_string();

    // 1. Create Address 1 (auto-default)
    let addr1_payload = json!({
        "recipient_name": "Alamat Rumah",
        "phone_number": "+62811111111",
        "street_address": "Jl. Mawar No. 1",
        "subdistrict": "Kecamatan Satu",
        "city": "Jakarta Selatan",
        "province": "DKI Jakarta",
        "postal_code": "12345",
        "set_as_default": false
    });
    let req = Request::builder()
        .uri("/api/v1/buyer/addresses")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&addr1_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let addr1: Value = serde_json::from_slice(&body).unwrap();
    let addr1_id = addr1["id"].as_str().unwrap().to_string();
    assert!(addr1["is_default"].as_bool().unwrap());

    // 2. Create Address 2 (explicit default)
    let addr2_payload = json!({
        "recipient_name": "Alamat Kantor",
        "phone_number": "+62822222222",
        "street_address": "Gedung Cyber 2 Lt 15",
        "subdistrict": "Kuningan Barat",
        "city": "Jakarta Selatan",
        "province": "DKI Jakarta",
        "postal_code": "12710",
        "set_as_default": true
    });
    let req = Request::builder()
        .uri("/api/v1/buyer/addresses")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&addr2_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let addr2: Value = serde_json::from_slice(&body).unwrap();
    assert!(addr2["is_default"].as_bool().unwrap());

    // 3. List addresses -> exactly one is default
    let req = Request::builder()
        .uri("/api/v1/buyer/addresses")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let list: Vec<Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(list.len(), 2);
    let default_count = list
        .iter()
        .filter(|a| a["is_default"].as_bool().unwrap())
        .count();
    assert_eq!(default_count, 1);

    // 4. Switch default back to Address 1
    let req = Request::builder()
        .uri(format!("/api/v1/buyer/addresses/{}/default", addr1_id))
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let switched: Value = serde_json::from_slice(&body).unwrap();
    assert!(switched["is_default"].as_bool().unwrap());
}

#[tokio::test]
async fn test_token_separation_buyer_cannot_access_seller_routes() {
    let (app, _, _) = setup_buyer_test_app().await;

    // Login as buyer
    let login_payload = json!({
        "id_token": "test_google:g_sub_sep:sep@buyer.com:Sep Buyer"
    });
    let req = Request::builder()
        .uri("/api/v1/buyer/auth/google")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&login_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let buyer_token = json["access_token"].as_str().unwrap().to_string();

    // 1. Buyer token on seller order list -> 403 Forbidden!
    let req = Request::builder()
        .uri("/api/v1/orders")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 2. Buyer token on seller accounts list -> 403 Forbidden!
    let req = Request::builder()
        .uri("/api/v1/users/accounts")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_token_separation_seller_cannot_access_buyer_routes() {
    let (app, _, _auth_module) = setup_buyer_test_app().await;

    // Login as admin/seller
    let login_payload = json!({
        "username": "admin",
        "password": "admin123"
    });
    let req = Request::builder()
        .uri("/api/v1/auth/login")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&login_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let seller_token = json["access_token"].as_str().unwrap().to_string();

    // Seller token accessing buyer address list -> 403 Forbidden!
    let req = Request::builder()
        .uri("/api/v1/buyer/addresses")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", seller_token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_break_glass_developer_support_lifecycle() {
    let (app, _, _) = setup_buyer_test_app().await;
    let dev_pass = "dev_support_secret_token_12345!";

    // 1. dev_support cannot login with admin123
    let wrong_pass_payload = json!({
        "username": "dev_support",
        "password": "admin123"
    });
    let req = Request::builder()
        .uri("/api/v1/auth/login")
        .method("POST")
        .header("x-forwarded-for", "192.168.1.101")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&wrong_pass_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 2. dev_support cannot login even with valid password when break-glass is inactive
    let dev_login_payload = json!({
        "username": "dev_support",
        "password": dev_pass
    });
    let req = Request::builder()
        .uri("/api/v1/auth/login")
        .method("POST")
        .header("x-forwarded-for", "192.168.1.102")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&dev_login_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 3. Operational admin (admin_ops) logs in
    let ops_login_payload = json!({
        "username": "admin_ops",
        "password": "admin123"
    });
    let req = Request::builder()
        .uri("/api/v1/auth/login")
        .method("POST")
        .header("x-forwarded-for", "192.168.1.103")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&ops_login_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let ops_json: Value = serde_json::from_slice(&body).unwrap();
    let ops_token = ops_json["access_token"].as_str().unwrap().to_string();

    // 4. Operational admin attempts to activate break-glass -> MUST BE REJECTED (403 Forbidden)!
    let activate_payload = json!({
        "reason": "Unauthorized attempt by operational admin",
        "duration_minutes": 60
    });
    let req = Request::builder()
        .uri("/api/v1/admin/break-glass/activate")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", ops_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&activate_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 5. Merchant Super Admin (Owner) logs in
    let admin_login_payload = json!({
        "username": "admin",
        "password": "admin123"
    });
    let req = Request::builder()
        .uri("/api/v1/auth/login")
        .method("POST")
        .header("x-forwarded-for", "192.168.1.104")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(&admin_login_payload).unwrap(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let admin_json: Value = serde_json::from_slice(&body).unwrap();
    let admin_token = admin_json["access_token"].as_str().unwrap().to_string();

    // 6. Super Admin activates break-glass -> 200 OK!
    let activate_payload = json!({
        "reason": "Production outage: database locks causing checkout timeout",
        "duration_minutes": 60
    });
    let req = Request::builder()
        .uri("/api/v1/admin/break-glass/activate")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&activate_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let status_json: Value = serde_json::from_slice(&body).unwrap();
    assert!(status_json["is_active"].as_bool().unwrap());

    // 7. Now dev_support can login with independent password!
    let req = Request::builder()
        .uri("/api/v1/auth/login")
        .method("POST")
        .header("x-forwarded-for", "192.168.1.105")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&dev_login_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let dev_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(dev_json["user"]["username"], "dev_support");
    assert_eq!(dev_json["user"]["role"], "Developer Support");
    let dev_token = dev_json["access_token"].as_str().unwrap().to_string();

    // Verify dev_support can access protected inventory API while break-glass is active
    let req = Request::builder()
        .uri("/api/v1/inventory")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", dev_token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 8. Admin deactivates break-glass
    let deactivate_payload = json!({
        "reason": "Incident resolved and database restarted"
    });
    let req = Request::builder()
        .uri("/api/v1/admin/break-glass/deactivate")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&deactivate_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 9. dev_support can no longer login
    let req = Request::builder()
        .uri("/api/v1/auth/login")
        .method("POST")
        .header("x-forwarded-for", "192.168.1.106")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&dev_login_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 10. dev_support token previously issued is IMMEDIATELY rejected after break-glass deactivation
    let req = Request::builder()
        .uri("/api/v1/inventory")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", dev_token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let err_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(err_json["error"], "token_revoked");
}

#[tokio::test]
async fn test_checkout_security_and_server_constructed_shipping_snapshot() {
    let (app, sent_otps, _) = setup_buyer_test_app().await;

    // Get catalog product ID
    let req = Request::builder()
        .uri("/api/v1/catalog")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let catalog_items: Vec<Value> = serde_json::from_slice(&body).unwrap();
    let product_id = catalog_items[0]["id"].as_str().unwrap().to_string();

    // 1. Unauthenticated checkout attempt -> 401 Unauthorized
    let unauth_payload = json!({
        "address_id": Uuid::new_v4(),
        "items": [{ "product_id": product_id, "quantity": 1 }]
    });
    let req = Request::builder()
        .uri("/api/v1/orders")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&unauth_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 2. Buyer A logs in via Google (phone not verified yet)
    let login_a = json!({ "id_token": "test_google:sub_a:buyer_a@gmail.com:Buyer A" });
    let req = Request::builder()
        .uri("/api/v1/buyer/auth/google")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&login_a).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let data_a: Value = serde_json::from_slice(&body).unwrap();
    let token_a = data_a["access_token"].as_str().unwrap().to_string();
    let buyer_a_id = data_a["buyer"]["id"].as_str().unwrap().to_string();

    // 3. Unverified Buyer A tries to checkout -> 422 Unprocessable Entity!
    let req = Request::builder()
        .uri("/api/v1/orders")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", token_a))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&unauth_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // 4. Buyer A verifies phone with OTP
    let phone_a = "+628123456789";
    let req = Request::builder()
        .uri("/api/v1/buyer/otp/request")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", token_a))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(&json!({ "phone_number": "08123456789" })).unwrap(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let otp_a = {
        let lock = sent_otps.read().await;
        lock.get(phone_a).cloned().expect("OTP must exist")
    };
    let req = Request::builder()
        .uri("/api/v1/buyer/otp/verify")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", token_a))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(&json!({ "phone_number": phone_a, "code": otp_a })).unwrap(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 5. Buyer A adds an address
    let addr_payload = json!({
        "recipient_name": "Rian Kusuma",
        "phone_number": "08123456789",
        "street_address": "Jl. Gatot Subroto No. 40",
        "subdistrict": "Kuningan",
        "city": "Jakarta Selatan",
        "province": "DKI Jakarta",
        "postal_code": "12950",
        "set_as_default": true
    });
    let req = Request::builder()
        .uri("/api/v1/buyer/addresses")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", token_a))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&addr_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let addr_a: Value = serde_json::from_slice(&body).unwrap();
    let addr_a_id = addr_a["id"].as_str().unwrap().to_string();

    // 6. Buyer A tries to checkout with a non-existent or foreign address ID -> 404 Not Found!
    let foreign_addr_payload = json!({
        "address_id": Uuid::new_v4(),
        "items": [{ "product_id": product_id, "quantity": 1 }]
    });
    let req = Request::builder()
        .uri("/api/v1/orders")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", token_a))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(&foreign_addr_payload).unwrap(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 7. Legitimate checkout with verified Buyer A and valid address -> 201 Created!
    let checkout_payload = json!({
        "address_id": addr_a_id,
        "items": [{ "product_id": product_id, "quantity": 2 }]
    });
    let req = Request::builder()
        .uri("/api/v1/orders")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", token_a))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&checkout_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let order_json: Value = serde_json::from_slice(&body).unwrap();

    // Verify buyer link and server-constructed immutable shipping snapshot
    assert_eq!(order_json["buyer_id"], buyer_a_id);
    assert_eq!(
        order_json["shipping_snapshot"]["recipient_name"],
        "Rian Kusuma"
    );
    assert_eq!(
        order_json["shipping_snapshot"]["phone_number"],
        "+628123456789"
    );
    assert_eq!(
        order_json["shipping_snapshot"]["street_address"],
        "Jl. Gatot Subroto No. 40"
    );
    assert_eq!(order_json["shipping_snapshot"]["postal_code"], "12950");
}

#[tokio::test]
async fn test_public_buyer_auth_config_endpoint() {
    let (app, _, _) = setup_buyer_test_app().await;

    // 1. GET /api/v1/buyer/auth/config
    let req = Request::builder()
        .uri("/api/v1/buyer/auth/config")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let config_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        config_json["google_client_id"],
        "test-google-client-id.apps.googleusercontent.com"
    );

    // 2. GET /api/v1/store/info also contains google_client_id
    let req = Request::builder()
        .uri("/api/v1/store/info")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let store_info: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        store_info["google_client_id"],
        "test-google-client-id.apps.googleusercontent.com"
    );
    assert_eq!(store_info["whatsapp_number"], "085810007735");
}

#[tokio::test]
async fn test_break_glass_token_rejected_on_expiry() {
    let (app, _, _, pool) = setup_buyer_test_app_with_pool().await;

    // 1. Admin logs in
    let admin_login = json!({
        "username": "admin",
        "password": "admin123"
    });
    let req = Request::builder()
        .uri("/api/v1/auth/login")
        .method("POST")
        .header("x-forwarded-for", "192.168.1.100")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&admin_login).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let admin_json: Value = serde_json::from_slice(&body).unwrap();
    let admin_token = admin_json["access_token"].as_str().unwrap().to_string();

    // 2. Set known password for dev_support in DB
    let dev_pass = "dev_support_secret_token_12345!";
    let hashed = program1_core::auth::hash_password(dev_pass).unwrap();
    sqlx::query("UPDATE user_accounts SET password_hash = $1 WHERE username = 'dev_support'")
        .bind(&hashed)
        .execute(&pool)
        .await
        .unwrap();

    // 3. Admin activates break glass
    let activate_payload = json!({
        "reason": "Test expiry scenario",
        "duration_minutes": 60
    });
    let req = Request::builder()
        .uri("/api/v1/admin/break-glass/activate")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&activate_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 4. dev_support logs in and obtains token
    let dev_login = json!({
        "username": "dev_support",
        "password": dev_pass
    });
    let req = Request::builder()
        .uri("/api/v1/auth/login")
        .method("POST")
        .header("x-forwarded-for", "192.168.1.101")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&dev_login).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let dev_json: Value = serde_json::from_slice(&body).unwrap();
    let dev_token = dev_json["access_token"].as_str().unwrap().to_string();

    // 5. Active token can access protected resource
    let req = Request::builder()
        .uri("/api/v1/inventory")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", dev_token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 6. Simulate session expiry by updating active_until to 10 minutes ago
    let past = chrono::Utc::now() - chrono::Duration::minutes(10);
    sqlx::query(
        "UPDATE break_glass_sessions SET active_until = $1 WHERE account_username = 'dev_support'",
    )
    .bind(past.to_rfc3339())
    .execute(&pool)
    .await
    .unwrap();

    // 7. Expired dev_token must be rejected with 401 and token_expired
    let req = Request::builder()
        .uri("/api/v1/inventory")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", dev_token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let err_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(err_json["error"], "token_expired");
}

#[tokio::test]
async fn test_checkout_negative_scenarios_comprehensive() {
    let (app, sent_otps, _, pool) = setup_buyer_test_app_with_pool().await;

    // Catalog item
    let req = Request::builder()
        .uri("/api/v1/catalog")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let catalog_items: Vec<Value> = serde_json::from_slice(&body).unwrap();
    let product_id = catalog_items[0]["id"].as_str().unwrap().to_string();

    // 1. Seller staff token attempting checkout -> 403 Forbidden
    let seller_login = json!({
        "username": "admin",
        "password": "admin123"
    });
    let req = Request::builder()
        .uri("/api/v1/auth/login")
        .method("POST")
        .header("x-forwarded-for", "192.168.1.100")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&seller_login).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let seller_json: Value = serde_json::from_slice(&body).unwrap();
    let seller_token = seller_json["access_token"].as_str().unwrap().to_string();

    let valid_payload = json!({
        "address_id": Uuid::new_v4(),
        "items": [{ "product_id": product_id, "quantity": 1 }]
    });
    let req = Request::builder()
        .uri("/api/v1/orders")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", seller_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&valid_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 2. Missing Authorization header -> 401 Unauthorized
    let req = Request::builder()
        .uri("/api/v1/orders")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&valid_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Setup Buyer 1 (with verified phone & address)
    let login_buyer1 = json!({ "id_token": "test_google:sub_buyer1:buyer1@test.com:Buyer One" });
    let req = Request::builder()
        .uri("/api/v1/buyer/auth/google")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&login_buyer1).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let data_buyer1: Value = serde_json::from_slice(&body).unwrap();
    let token_buyer1 = data_buyer1["access_token"].as_str().unwrap().to_string();
    let buyer1_id = data_buyer1["buyer"]["id"].as_str().unwrap().to_string();

    // 3. Unverified phone -> 422 Unprocessable Entity
    let req = Request::builder()
        .uri("/api/v1/orders")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", token_buyer1))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&valid_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Verify phone for Buyer 1
    let phone1 = "+6281234567890";
    let req = Request::builder()
        .uri("/api/v1/buyer/otp/request")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", token_buyer1))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(&json!({ "phone_number": phone1 })).unwrap(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let otp1 = {
        let lock = sent_otps.read().await;
        lock.get(phone1).cloned().unwrap()
    };
    let req = Request::builder()
        .uri("/api/v1/buyer/otp/verify")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", token_buyer1))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(&json!({ "phone_number": phone1, "code": otp1 })).unwrap(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Add address for Buyer 1
    let addr_payload = json!({
        "recipient_name": "Buyer One Recipient",
        "phone_number": phone1,
        "street_address": "Jl. Merdeka 1",
        "subdistrict": "Gambir",
        "city": "Jakarta Pusat",
        "province": "DKI Jakarta",
        "postal_code": "10110",
        "set_as_default": true
    });
    let req = Request::builder()
        .uri("/api/v1/buyer/addresses")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", token_buyer1))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&addr_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let addr1_json: Value = serde_json::from_slice(&body).unwrap();
    let addr1_id = addr1_json["id"].as_str().unwrap().to_string();

    // 4. Non-existent address -> 404 Not Found
    let fake_addr_payload = json!({
        "address_id": Uuid::new_v4(),
        "items": [{ "product_id": product_id, "quantity": 1 }]
    });
    let req = Request::builder()
        .uri("/api/v1/orders")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", token_buyer1))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&fake_addr_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // Setup Buyer 2 (foreign buyer)
    let login_buyer2 = json!({ "id_token": "test_google:sub_buyer2:buyer2@test.com:Buyer Two" });
    let req = Request::builder()
        .uri("/api/v1/buyer/auth/google")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&login_buyer2).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let data_buyer2: Value = serde_json::from_slice(&body).unwrap();
    let token_buyer2 = data_buyer2["access_token"].as_str().unwrap().to_string();

    // Verify phone for Buyer 2
    let phone2 = "+6281234567899";
    let req = Request::builder()
        .uri("/api/v1/buyer/otp/request")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", token_buyer2))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(&json!({ "phone_number": phone2 })).unwrap(),
        ))
        .unwrap();
    let _ = app.clone().oneshot(req).await.unwrap();
    let otp2 = {
        let lock = sent_otps.read().await;
        lock.get(phone2).cloned().unwrap()
    };
    let req = Request::builder()
        .uri("/api/v1/buyer/otp/verify")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", token_buyer2))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(&json!({ "phone_number": phone2, "code": otp2 })).unwrap(),
        ))
        .unwrap();
    let _ = app.clone().oneshot(req).await.unwrap();

    // 5. Foreign address: Buyer 2 tries to use Buyer 1's address -> 404 Not Found
    let foreign_addr_req = json!({
        "address_id": addr1_id,
        "items": [{ "product_id": product_id, "quantity": 1 }]
    });
    let req = Request::builder()
        .uri("/api/v1/orders")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", token_buyer2))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&foreign_addr_req).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 6. Empty items array -> 422 Unprocessable Entity
    let empty_items_payload = json!({
        "address_id": addr1_id,
        "items": []
    });
    let req = Request::builder()
        .uri("/api/v1/orders")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", token_buyer1))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(&empty_items_payload).unwrap(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // 7. Zero quantity item -> 422 Unprocessable Entity
    let zero_qty_payload = json!({
        "address_id": addr1_id,
        "items": [{ "product_id": product_id, "quantity": 0 }]
    });
    let req = Request::builder()
        .uri("/api/v1/orders")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", token_buyer1))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&zero_qty_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // 8. Inactive buyer -> 403 Forbidden
    sqlx::query("UPDATE buyer_accounts SET is_active = 0 WHERE id = $1")
        .bind(&buyer1_id)
        .execute(&pool)
        .await
        .unwrap();

    let valid_checkout = json!({
        "address_id": addr1_id,
        "items": [{ "product_id": product_id, "quantity": 1 }]
    });
    let req = Request::builder()
        .uri("/api/v1/orders")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", token_buyer1))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&valid_checkout).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let err_body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(err_body["error"], "account_deactivated");
}

#[tokio::test]
async fn test_buyer_register_and_login_api_endpoints() {
    let (app, _sent_otps, _auth_module, _pool) = setup_buyer_test_app_with_pool().await;

    // 1. Register a new buyer
    let reg_payload = json!({
        "full_name": "Rina Wijaya",
        "email": "rina@example.com",
        "password": "Password123!"
    });
    let req = Request::builder()
        .uri("/api/v1/buyer/auth/register")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&reg_payload).unwrap()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let reg_data: Value = serde_json::from_slice(&body).unwrap();
    let token = reg_data["access_token"].as_str().unwrap();
    assert!(!token.is_empty());
    assert_eq!(reg_data["buyer"]["email"], "rina@example.com");
    assert_eq!(reg_data["buyer"]["full_name"], "Rina Wijaya");
    assert_eq!(reg_data["requires_phone_verification"], true);

    // 2. Duplicate registration -> 400 Bad Request
    let req_dup = Request::builder()
        .uri("/api/v1/buyer/auth/register")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&reg_payload).unwrap()))
        .unwrap();

    let res_dup = app.clone().oneshot(req_dup).await.unwrap();
    assert_eq!(res_dup.status(), StatusCode::BAD_REQUEST);

    // 3. Login with correct credentials
    let login_payload = json!({
        "email": "rina@example.com",
        "password": "Password123!"
    });
    let req_login = Request::builder()
        .uri("/api/v1/buyer/auth/login")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&login_payload).unwrap()))
        .unwrap();

    let res_login = app.clone().oneshot(req_login).await.unwrap();
    assert_eq!(res_login.status(), StatusCode::OK);
    let body = to_bytes(res_login.into_body(), usize::MAX).await.unwrap();
    let login_data: Value = serde_json::from_slice(&body).unwrap();
    let login_token = login_data["access_token"].as_str().unwrap();
    assert!(!login_token.is_empty());

    // 4. Login with wrong password -> 400 Bad Request
    let wrong_login_payload = json!({
        "email": "rina@example.com",
        "password": "WrongPassword999!"
    });
    let req_wrong = Request::builder()
        .uri("/api/v1/buyer/auth/login")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(&wrong_login_payload).unwrap(),
        ))
        .unwrap();

    let res_wrong = app.clone().oneshot(req_wrong).await.unwrap();
    assert_eq!(res_wrong.status(), StatusCode::BAD_REQUEST);

    // 5. Use token to get buyer profile
    let req_profile = Request::builder()
        .uri("/api/v1/buyer/profile")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", login_token))
        .body(Body::empty())
        .unwrap();

    let res_profile = app.clone().oneshot(req_profile).await.unwrap();
    assert_eq!(res_profile.status(), StatusCode::OK);
    let body = to_bytes(res_profile.into_body(), usize::MAX).await.unwrap();
    let profile_data: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(profile_data["email"], "rina@example.com");
    assert_eq!(profile_data["full_name"], "Rina Wijaya");
}

#[tokio::test]
async fn test_admin_buyers_management_and_activity() {
    let (app, _sent_otps, auth_module, _pool) = setup_buyer_test_app_with_pool().await;

    // 1. Register a buyer
    let reg_payload = json!({
        "full_name": "Dewi Sartika",
        "email": "dewi@example.com",
        "password": "Password123!",
        "phone_number": "081234567890"
    });
    let req_reg = Request::builder()
        .uri("/api/v1/buyer/auth/register")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&reg_payload).unwrap()))
        .unwrap();

    let res_reg = app.clone().oneshot(req_reg).await.unwrap();
    assert_eq!(res_reg.status(), StatusCode::CREATED);
    let body = to_bytes(res_reg.into_body(), usize::MAX).await.unwrap();
    let reg_data: Value = serde_json::from_slice(&body).unwrap();
    let buyer_id = reg_data["buyer"]["id"].as_str().unwrap();
    let buyer_token = reg_data["access_token"].as_str().unwrap();

    // 2. Accessing admin endpoints without token -> 401 Unauthorized
    let req_unauth = Request::builder()
        .uri("/api/v1/admin/buyers")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res_unauth = app.clone().oneshot(req_unauth).await.unwrap();
    assert_eq!(res_unauth.status(), StatusCode::UNAUTHORIZED);

    // 3. Accessing admin endpoints with buyer token -> 403 Forbidden
    let req_forbidden = Request::builder()
        .uri("/api/v1/admin/buyers")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
        .body(Body::empty())
        .unwrap();
    let res_forbidden = app.clone().oneshot(req_forbidden).await.unwrap();
    assert_eq!(res_forbidden.status(), StatusCode::FORBIDDEN);

    // 4. Create seller staff token
    let staff_claims = UserAccountDto {
        id: Uuid::new_v4(),
        username: "admin_seller".to_string(),
        full_name: "Admin Seller".to_string(),
        role: "Admin".to_string(),
        accessible_menus: vec!["customers".to_string()],
        is_active: true,
        created_at: Utc::now(),
    };
    let staff_token = auth_module.generate_token(&staff_claims).unwrap();

    // 5. Seller staff lists all buyers -> 200 OK
    let req_list = Request::builder()
        .uri("/api/v1/admin/buyers")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", staff_token))
        .body(Body::empty())
        .unwrap();
    let res_list = app.clone().oneshot(req_list).await.unwrap();
    assert_eq!(res_list.status(), StatusCode::OK);
    let body = to_bytes(res_list.into_body(), usize::MAX).await.unwrap();
    let buyers_data: Value = serde_json::from_slice(&body).unwrap();
    let buyers_array = buyers_data.as_array().unwrap();
    assert!(buyers_array
        .iter()
        .any(|b| b["email"] == "dewi@example.com"));

    // 6. Seller updates buyer active status to false -> 200 OK
    let update_status_payload = json!({
        "is_active": false
    });
    let req_update_status = Request::builder()
        .uri(format!("/api/v1/admin/buyers/{}/status", buyer_id))
        .method("PATCH")
        .header(header::AUTHORIZATION, format!("Bearer {}", staff_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(&update_status_payload).unwrap(),
        ))
        .unwrap();
    let res_update = app.clone().oneshot(req_update_status).await.unwrap();
    assert_eq!(res_update.status(), StatusCode::OK);
    let body = to_bytes(res_update.into_body(), usize::MAX).await.unwrap();
    let updated_data: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(updated_data["is_active"], false);

    // 7. Seller checks buyer activity logs -> 200 OK
    let req_act = Request::builder()
        .uri("/api/v1/admin/buyers/activity")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", staff_token))
        .body(Body::empty())
        .unwrap();
    let res_act = app.clone().oneshot(req_act).await.unwrap();
    assert_eq!(res_act.status(), StatusCode::OK);
    let body = to_bytes(res_act.into_body(), usize::MAX).await.unwrap();
    let act_data: Value = serde_json::from_slice(&body).unwrap();
    let act_array = act_data.as_array().unwrap();
    assert!(act_array
        .iter()
        .any(|l| l["action"] == "BUYER_REGISTERED_EMAIL"));
}
