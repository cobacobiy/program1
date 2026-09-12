use std::sync::Arc;
use std::time::Duration;

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

use program1_contracts::{
    AuthContract, BuyerContract, CatalogContract, OrderContract, OrderStatus, RegisterBuyerRequest,
    StorefrontOrderItemRequest, StorefrontOrderRequest, UserContract,
};
use program1_core::init_database;
use program1_core::MockEmailSender;
use program1_module_analytics::AnalyticsModule;
use program1_module_audit::AuditModule;
use program1_module_auth::AuthModule;
use program1_module_buyer::BuyerModule;
use program1_module_catalog::CatalogModule;
use program1_module_channel::ChannelSyncModule;
use program1_module_chat::ChatModule;
use program1_module_inventory::InventoryModule;
use program1_module_order::OrderModule;
use program1_module_payment::PaymentModule;
use program1_module_user::UserModule;
use program1_web::{create_app, AppState};

#[tokio::test]
async fn test_buyer_registration_sends_welcome_email() {
    let pool = init_database("sqlite::memory:")
        .await
        .expect("Failed to init db");

    let auth_module = Arc::new(AuthModule::new(
        "jwt-secret-1234567890-test-key-32chars".to_string(),
        24,
    ));
    let audit_module = Arc::new(AuditModule::new(pool.clone()));
    let google_verifier = Arc::new(program1_module_buyer::ProductionGoogleVerifier {
        client_id: "test".to_string(),
    });
    let sms_sender = Arc::new(program1_module_buyer::ConsoleOrProviderSmsSender::new(
        "test".to_string(),
        "console".to_string(),
        "".to_string(),
    ));

    let mock_email = Arc::new(MockEmailSender::new());

    let buyer_module = BuyerModule::new(
        pool.clone(),
        auth_module.clone(),
        google_verifier,
        sms_sender,
        audit_module.clone(),
    )
    .with_email_sender(mock_email.clone(), "AURA Test Store");

    let reg_req = RegisterBuyerRequest {
        full_name: "Ahmad Dahlan".to_string(),
        email: "ahmad@example.com".to_string(),
        password: "password123".to_string(),
    };

    let auth_res = buyer_module
        .register(reg_req)
        .await
        .expect("Registration failed");
    assert_eq!(auth_res.buyer.email, "ahmad@example.com");

    // Allow async background task to complete
    tokio::time::sleep(Duration::from_millis(60)).await;

    let sent = mock_email.get_sent_emails().await;
    assert_eq!(sent.len(), 1, "Expected exactly 1 welcome email");
    assert_eq!(sent[0].to, "ahmad@example.com");
    assert!(sent[0]
        .subject
        .contains("Selamat Datang di AURA Test Store"));
    assert!(sent[0].html_body.contains("Ahmad Dahlan"));
}

#[tokio::test]
async fn test_storefront_order_lifecycle_emails() {
    let pool = init_database("sqlite::memory:")
        .await
        .expect("Failed to init db");

    let catalog_module = Arc::new(CatalogModule::new(pool.clone()));
    catalog_module.seed_default_catalog().await.unwrap();

    let inventory_module = Arc::new(InventoryModule::new(pool.clone(), catalog_module.clone()));
    let mock_email = Arc::new(MockEmailSender::new());

    let order_module = OrderModule::new(
        pool.clone(),
        catalog_module.clone(),
        inventory_module.clone(),
    )
    .with_email_sender(mock_email.clone(), "AURA Boutique");

    let products = catalog_module.list_items().await.unwrap();
    assert!(!products.is_empty());

    // 1. Create storefront order -> triggers Order Confirmation Email
    let order_req = StorefrontOrderRequest {
        customer_name: "Dewi Sartika".to_string(),
        customer_email: "dewi.sartika@example.com".to_string(),
        shipping_address: "Jl. Asia Afrika No. 12, Bandung".to_string(),
        items: vec![StorefrontOrderItemRequest {
            product_id: products[0].id,
            quantity: 2,
        }],
        buyer_id: None,
        shipping_snapshot: None,
        courier: None,
        shipping_cost_cents: None,
    };

    let order = order_module
        .create_storefront_order(order_req)
        .await
        .expect("Failed to create order");

    tokio::time::sleep(Duration::from_millis(60)).await;

    let sent_1 = mock_email.get_sent_emails().await;
    assert_eq!(sent_1.len(), 1, "Expected order confirmation email");
    assert_eq!(sent_1[0].to, "dewi.sartika@example.com");
    assert!(sent_1[0].subject.contains("Konfirmasi Pesanan"));
    assert!(sent_1[0].html_body.contains("Dewi Sartika"));
    assert!(sent_1[0].html_body.contains(&products[0].name));

    // 2. Transition Pending -> Paid -> triggers Payment Success Email
    let paid_order = order_module
        .update_order_status_with_metadata(
            order.id,
            OrderStatus::Paid,
            "midtrans_webhook",
            None,
            None,
        )
        .await
        .expect("Failed to update status to Paid");
    assert_eq!(paid_order.status, "paid");

    tokio::time::sleep(Duration::from_millis(60)).await;

    let sent_2 = mock_email.get_sent_emails().await;
    assert_eq!(sent_2.len(), 2, "Expected payment success email added");
    assert_eq!(sent_2[1].to, "dewi.sartika@example.com");
    assert!(sent_2[1].subject.contains("Pembayaran Berhasil"));
    assert!(sent_2[1]
        .html_body
        .contains("Pembayaran Berhasil Dikonfirmasi"));

    // 3. Transition Paid -> Processing (no email configured for processing)
    let processing_order = order_module
        .update_order_status_with_metadata(
            order.id,
            OrderStatus::Processing,
            "warehouse_staff",
            None,
            None,
        )
        .await
        .expect("Failed to update status to Processing");
    assert_eq!(processing_order.status, "processing");

    tokio::time::sleep(Duration::from_millis(40)).await;
    assert_eq!(mock_email.get_sent_emails().await.len(), 2);

    // 4. Transition Processing -> Shipped -> triggers Shipping Notification Email
    let shipped_order = order_module
        .update_order_status_with_metadata(
            order.id,
            OrderStatus::Shipped,
            "seller_admin",
            Some("SICEPAT-RES-001122".to_string()),
            None,
        )
        .await
        .expect("Failed to update status to Shipped");
    assert_eq!(shipped_order.status, "shipped");

    tokio::time::sleep(Duration::from_millis(60)).await;

    let sent_3 = mock_email.get_sent_emails().await;
    assert_eq!(
        sent_3.len(),
        3,
        "Expected shipping notification email added"
    );
    assert_eq!(sent_3[2].to, "dewi.sartika@example.com");
    assert!(sent_3[2].subject.contains("Sedang Dalam Pengiriman"));
    assert!(sent_3[2].html_body.contains("SICEPAT-RES-001122"));
}

#[tokio::test]
async fn test_full_http_storefront_checkout_email_flow() {
    let pool = init_database("sqlite::memory:")
        .await
        .expect("Failed to init db");

    let user_module = Arc::new(UserModule::new(pool.clone()));
    let auth_module = Arc::new(AuthModule::new(
        "jwt-secret-for-http-tests-32chars-min!!".to_string(),
        24,
    ));
    let catalog_module = Arc::new(CatalogModule::new(pool.clone()));
    let inventory_module = Arc::new(InventoryModule::new(pool.clone(), catalog_module.clone()));
    let channel_module = Arc::new(ChannelSyncModule::new(pool.clone()));

    let mock_email = Arc::new(MockEmailSender::new());

    let order_module = Arc::new(
        OrderModule::new(
            pool.clone(),
            catalog_module.clone(),
            inventory_module.clone(),
        )
        .with_email_sender(mock_email.clone(), "HTTP Test Store"),
    );

    let analytics_module = Arc::new(AnalyticsModule::new(
        catalog_module.clone(),
        order_module.clone(),
    ));
    let audit_module = Arc::new(AuditModule::new(pool.clone()));

    let google_verifier = Arc::new(program1_module_buyer::ProductionGoogleVerifier {
        client_id: "test".to_string(),
    });
    let sms_sender = Arc::new(program1_module_buyer::ConsoleOrProviderSmsSender::new(
        "test".to_string(),
        "console".to_string(),
        "".to_string(),
    ));

    let buyer_module = Arc::new(
        BuyerModule::new(
            pool.clone(),
            auth_module.clone(),
            google_verifier,
            sms_sender,
            audit_module.clone(),
        )
        .with_email_sender(mock_email.clone(), "HTTP Test Store"),
    );

    let chat_module = Arc::new(ChatModule::new(pool.clone()));
    let payment_module = Arc::new(PaymentModule::new(
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

    let admin_user = user_module.authenticate("admin", "admin123").await.unwrap();
    let admin_token = auth_module.generate_token(&admin_user).unwrap();

    let state = AppState {
        store_name: "HTTP Test Store".to_string(),
        store_currency: "IDR".to_string(),
        store_whatsapp_number: "085810007735".to_string(),
        user_contract: user_module,
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
        shipping_contract: Arc::new(program1_module_shipping::ShippingModule::new("".to_string(), "starter".to_string(), "152".to_string())),
        notification_contract: Arc::new(program1_module_notification::NotificationModule::new(pool.clone())),
        return_contract: Arc::new(program1_module_return::ReturnModule::new(pool.clone())),
        backup_contract: Arc::new(program1_core::backup::BackupService::new(pool.clone(), "./target/test_backups")),
        rate_limiter: Arc::new(program1_web::rate_limit::IpRateLimiter::new()),
        started_at: std::time::Instant::now(),
        google_client_id: "test".to_string(),
    };

    let app = create_app(state);

    // 1. Buyer register via HTTP -> receives welcome email
    let reg_body = json!({
        "full_name": "Rina Nose",
        "email": "rina.nose@example.com",
        "password": "password123"
    });

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/buyer/auth/register")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(reg_body.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let reg_bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let reg_val: serde_json::Value = serde_json::from_slice(&reg_bytes).unwrap();
    let buyer_token = reg_val["access_token"].as_str().unwrap();
    let buyer_id = reg_val["buyer"]["id"].as_str().unwrap();

    // Set buyer phone verified and create address
    sqlx::query("UPDATE buyer_accounts SET phone_verified = 1, phone_number = '+628123456789' WHERE id = $1")
        .bind(buyer_id)
        .execute(&pool)
        .await
        .unwrap();

    let addr_id = uuid::Uuid::new_v4();
    let now_str = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO buyer_addresses (id, buyer_id, recipient_name, phone_number, street_address, subdistrict, city, province, postal_code, is_default, created_at, updated_at) \
         VALUES ($1, $2, 'Rina Nose', '+628123456789', 'Jl. Teuku Umar No. 5', 'Denpasar Barat', 'Denpasar', 'Bali', '80113', 1, $3, $4)",
    )
    .bind(addr_id.to_string())
    .bind(buyer_id)
    .bind(&now_str)
    .bind(&now_str)
    .execute(&pool)
    .await
    .unwrap();

    tokio::time::sleep(Duration::from_millis(60)).await;

    let sent = mock_email.get_sent_emails().await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, "rina.nose@example.com");
    assert!(sent[0].subject.contains("Selamat Datang"));

    // 2. Buyer places order via HTTP -> receives order confirmation email
    let products = catalog_module.list_items().await.unwrap();
    let order_body = json!({
        "address_id": addr_id,
        "items": [
            {
                "product_id": products[0].id,
                "quantity": 1
            }
        ]
    });

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/orders")
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(order_body.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body_bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let order_val: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let order_id = order_val["id"].as_str().unwrap();

    tokio::time::sleep(Duration::from_millis(60)).await;

    let sent = mock_email.get_sent_emails().await;
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[1].to, "rina.nose@example.com");
    assert!(sent[1].subject.contains("Konfirmasi Pesanan"));

    // 3. Admin updates order to Paid via HTTP
    let patch_paid = json!({
        "status": "paid"
    });
    let req = Request::builder()
        .method(Method::PATCH)
        .uri(format!("/api/v1/orders/{}/status", order_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(patch_paid.to_string()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    tokio::time::sleep(Duration::from_millis(60)).await;

    let sent = mock_email.get_sent_emails().await;
    assert_eq!(sent.len(), 3);
    assert!(sent[2].subject.contains("Pembayaran Berhasil"));

    // 4. Admin updates order to Processing, then Shipped with tracking number
    let patch_processing = json!({
        "status": "processing"
    });
    let req = Request::builder()
        .method(Method::PATCH)
        .uri(format!("/api/v1/orders/{}/status", order_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(patch_processing.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let patch_shipped = json!({
        "status": "shipped",
        "tracking_number": "JNE-CARGO-777888"
    });
    let req = Request::builder()
        .method(Method::PATCH)
        .uri(format!("/api/v1/orders/{}/status", order_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(patch_shipped.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    tokio::time::sleep(Duration::from_millis(60)).await;

    let sent = mock_email.get_sent_emails().await;
    assert_eq!(sent.len(), 4);
    assert!(sent[3].subject.contains("Sedang Dalam Pengiriman"));
    assert!(sent[3].html_body.contains("JNE-CARGO-777888"));
}
