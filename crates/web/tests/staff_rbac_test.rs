use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
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
use program1_module_flash_sale::FlashSaleModule;
use program1_module_inventory::InventoryModule;
use program1_module_order::OrderModule;
use program1_module_payment::PaymentModule;
use program1_module_review::ReviewModule;
use program1_module_shipping::ShippingModule;
use program1_module_supplier::SupplierModule;
use program1_module_user::UserModule;
use program1_web::{create_app, state::AppState};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

async fn setup_test_app() -> (
    axum::Router,
    Arc<dyn UserContract>,
    Arc<dyn OrderContract>,
    Arc<dyn CatalogContract>,
    String, // secret
) {
    let secret = "super-secret-jwt-signing-key-for-tests-32chars!".to_string();
    let pool = init_database("sqlite::memory:").await.unwrap();

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
    let flash_sale_module = Arc::new(FlashSaleModule::new(pool.clone()));
    let supplier_module = Arc::new(SupplierModule::new(pool.clone()));

    user_module.seed_default_users().await.unwrap();
    catalog_module.seed_default_catalog().await.unwrap();

    let state = AppState {
        store_name: "AURA Storefront".to_string(),
        store_currency: "IDR".to_string(),
        store_whatsapp_number: "+6281234567890".to_string(),
        user_contract: user_module.clone(),
        auth_contract: auth_module,
        catalog_contract: catalog_module.clone(),
        inventory_contract: inventory_module,
        channel_contract: channel_module,
        order_contract: order_module.clone(),
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
        flash_sale_contract: flash_sale_module,
        supplier_contract: supplier_module,
        rate_limiter: Arc::new(program1_web::rate_limit::IpRateLimiter::new()),
        started_at: std::time::Instant::now(),
        google_client_id: "test".to_string(),
    };

    let app = create_app(state);
    (app, user_module, order_module, catalog_module, secret)
}

fn create_token_for_user(secret: &str, role: &str, perms: Vec<&str>) -> String {
    let now_ts = Utc::now().timestamp();
    let claims = JwtClaims {
        sub: Uuid::new_v4(),
        username: "staff_test".to_string(),
        role: role.to_string(),
        accessible_menus: vec!["orders".to_string()],
        exp: now_ts + 3600,
        iat: now_ts,
        user_type: "seller_staff".to_string(),
        permissions: perms.into_iter().map(String::from).collect(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap()
}

#[tokio::test]
async fn test_staff_lacking_permission_rejected_with_403() {
    let (app, _, _, _, secret) = setup_test_app().await;

    // Staff only has 'chat:support' permission
    let token = create_token_for_user(&secret, "Staff", vec!["chat:support"]);

    // Attempt to mutate catalog (requires 'catalog:write')
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/catalog")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "name": "Keyboard Mechanical Baru",
                "sku": "KB-NEW-01",
                "category": "Peripherals",
                "price": 500000.0,
                "stock": 10
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let body_bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body_json["error"], "insufficient_permissions");
    assert!(body_json["message"].as_str().unwrap().contains("catalog:write"));
}

#[tokio::test]
async fn test_staff_with_matching_permission_allowed() {
    let (app, _, order_contract, catalog_contract, secret) = setup_test_app().await;

    // Seed an order to mutate
    let items = catalog_contract.list_items().await.unwrap();
    let order = order_contract
        .create_storefront_order(StorefrontOrderRequest {
            customer_name: "Customer Satu".to_string(),
            customer_email: "cust1@example.com".to_string(),
            shipping_address: "Jl. Merdeka No 5".to_string(),
            items: vec![StorefrontOrderItemRequest::new(items[0].id, 1)],
            buyer_id: None,
            shipping_snapshot: None,
            courier: None,
            shipping_cost_cents: None,
            use_points: None,
        })
        .await
        .unwrap();

    // Staff has 'orders:manage' permission
    let token = create_token_for_user(&secret, "Staff", vec!["orders:manage"]);

    let req = Request::builder()
        .method("PATCH")
        .uri(format!("/api/v1/orders/{}/status", order.id))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "status": "paid"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_wildcard_permission_bypass() {
    let (app, _, _, _, secret) = setup_test_app().await;

    // Admin token (role = "admin", permissions = ["*"])
    let token = create_token_for_user(&secret, "Admin", vec!["*"]);

    // Admin can create catalog item (catalog:write)
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/catalog")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "name": "Headset Gaming Wireless Pro",
                "sku": "SKU-HEADSET-PRO-01",
                "category": "Peripherals",
                "price": 850000.0,
                "stock": 20
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_staff_permissions_management_api() {
    let (app, user_contract, _, _, secret) = setup_test_app().await;

    let admin_token = create_token_for_user(&secret, "Super Admin", vec!["*"]);

    // 1. List available system permissions
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/admin/permissions")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let perms: Vec<PermissionDto> = serde_json::from_slice(&body).unwrap();
    assert!(perms.iter().any(|p| p.name == "orders:manage"));
    assert!(perms.iter().any(|p| p.name == "catalog:write"));
    assert!(perms.iter().any(|p| p.name == "suppliers:manage"));

    // 2. Create staff account and set permissions
    let staff = user_contract
        .create_account(CreateUserAccountRequest {
            username: "staff_operasional".to_string(),
            full_name: "Staff Operasional".to_string(),
            role: "Staff".to_string(),
            accessible_menus: vec!["orders".to_string()],
            permissions: Some(vec!["orders:manage".to_string()]),
        })
        .await
        .unwrap();

    assert_eq!(staff.permissions, vec!["orders:manage"]);

    // 3. Update staff permissions via API
    let req = Request::builder()
        .method("PUT")
        .uri(format!(
            "/api/v1/users/accounts/{}/staff-permissions",
            staff.id
        ))
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "permissions": ["orders:manage", "inventory:manage", "suppliers:manage"]
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 4. Verify updated permissions
    let user_perms = user_contract.get_user_permissions(staff.id).await.unwrap();
    assert_eq!(user_perms.len(), 3);
    assert!(user_perms.contains(&"suppliers:manage".to_string()));
}
