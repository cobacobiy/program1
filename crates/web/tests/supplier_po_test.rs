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
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

async fn setup_test_app() -> (
    axum::Router,
    Arc<dyn SupplierContract>,
    Arc<dyn CatalogContract>,
    String, // admin token
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

    catalog_module.seed_default_catalog().await.unwrap();

    let state = AppState {
        store_name: "AURA Storefront".to_string(),
        store_currency: "IDR".to_string(),
        store_whatsapp_number: "+6281234567890".to_string(),
        user_contract: user_module,
        auth_contract: auth_module,
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
        supplier_contract: supplier_module.clone(),
        rate_limiter: Arc::new(program1_web::rate_limit::IpRateLimiter::new()),
        started_at: std::time::Instant::now(),
        google_client_id: "test".to_string(),
    };

    let app = create_app(state);

    let now_ts = Utc::now().timestamp();
    let admin_claims = JwtClaims {
        sub: Uuid::new_v4(),
        username: "admin".to_string(),
        role: "admin".to_string(),
        accessible_menus: vec!["all".to_string()],
        exp: now_ts + 3600,
        iat: now_ts,
        user_type: "seller_staff".to_string(),
        permissions: vec!["*".to_string()],
    };
    let admin_token = encode(
        &Header::default(),
        &admin_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap();

    (app, supplier_module, catalog_module, admin_token)
}

#[tokio::test]
async fn test_supplier_crud() {
    let (app, _, _, admin_token) = setup_test_app().await;

    // 1. Create Supplier
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/suppliers")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "name": "PT Sumber Logistik Elektronik",
                "contact_person": "Budi Hartono",
                "phone": "081234567890",
                "email": "budi@sumberlogistik.co.id",
                "address": "Kawasan Industri MM2100, Cikarang"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let supplier: SupplierDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(supplier.name, "PT Sumber Logistik Elektronik");
    assert!(supplier.is_active);

    // 2. List Suppliers
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/admin/suppliers")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let list: Vec<SupplierDto> = serde_json::from_slice(&body).unwrap();
    assert!(!list.is_empty());

    // 3. Update Supplier
    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/admin/suppliers/{}", supplier.id))
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "name": "PT Sumber Logistik Prima",
                "contact_person": "Budi Hartono (Senior Manager)",
                "phone": "081234567890",
                "email": "budi@sumberlogistik.co.id",
                "address": "Kawasan Industri MM2100, Cikarang",
                "is_active": true
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let updated: SupplierDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(updated.name, "PT Sumber Logistik Prima");

    // 4. Delete Supplier
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/admin/suppliers/{}", supplier.id))
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_purchase_order_creation_and_lifecycle() {
    let (app, supplier_contract, catalog_contract, admin_token) = setup_test_app().await;

    // Create supplier
    let supplier = supplier_contract
        .create_supplier(CreateSupplierRequest {
            name: "Distributor Resmi Keyboard".to_string(),
            contact_person: Some("Andi".to_string()),
            phone: Some("0811223344".to_string()),
            email: Some("andi@keyboard.com".to_string()),
            address: Some("Mangga Dua Mall".to_string()),
        })
        .await
        .unwrap();

    let items = catalog_contract.list_items().await.unwrap();
    let product_id = items[0].id.clone();

    // Create PO
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/purchase-orders")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "supplier_id": supplier.id,
                "expected_delivery_date": "2026-10-15",
                "notes": "Pengadaan batch Q4 2026",
                "items": [
                    {
                        "product_id": product_id.to_string(),
                        "quantity": 30,
                        "unit_cost_cents": 85000000
                    }
                ]
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let po: PurchaseOrderDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(po.status, "ordered");
    assert_eq!(po.items.len(), 1);
    assert_eq!(po.items[0].quantity, 30);
    assert_eq!(po.total_cost_cents, 30 * 85000000);

    // Cancel PO
    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/admin/purchase-orders/{}/cancel", po.id))
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let cancelled_po: PurchaseOrderDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(cancelled_po.status, "cancelled");
}

#[tokio::test]
async fn test_purchase_order_receive_and_restock() {
    let (app, supplier_contract, catalog_contract, admin_token) = setup_test_app().await;

    let supplier = supplier_contract
        .create_supplier(CreateSupplierRequest {
            name: "Pabrik Mouse Gaming".to_string(),
            contact_person: Some("Candra".to_string()),
            phone: Some("0899887766".to_string()),
            email: Some("candra@mouse.com".to_string()),
            address: Some("Tangerang".to_string()),
        })
        .await
        .unwrap();

    let items = catalog_contract.list_items().await.unwrap();
    let product_id = items[0].id;
    let initial_stock = items[0].stock;

    // Create PO for 25 units
    let po = supplier_contract
        .create_purchase_order(CreatePurchaseOrderRequest {
            supplier_id: supplier.id,
            expected_delivery_date: Some("2026-11-01".to_string()),
            notes: Some("Restock Gudang Utama".to_string()),
            items: vec![PurchaseOrderItemRequest {
                product_id: product_id.to_string(),
                variant_id: None,
                quantity: 25,
                unit_cost_cents: 50000000,
            }],
        })
        .await
        .unwrap();

    assert_eq!(po.status, "ordered");

    // Receive PO via REST endpoint
    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/admin/purchase-orders/{}/receive", po.id))
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let received_po: PurchaseOrderDto = serde_json::from_slice(&body).unwrap();
    assert_eq!(received_po.status, "received");

    // Verify catalog stock incremented by exactly 25
    let updated_items = catalog_contract.list_items().await.unwrap();
    let updated_product = updated_items.iter().find(|i| i.id == product_id).unwrap();
    assert_eq!(updated_product.stock, initial_stock + 25);

    // Attempting to receive again should be rejected with 422
    let req_again = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/admin/purchase-orders/{}/receive", po.id))
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();

    let resp_again = app.clone().oneshot(req_again).await.unwrap();
    assert_eq!(resp_again.status(), StatusCode::BAD_REQUEST);
}
