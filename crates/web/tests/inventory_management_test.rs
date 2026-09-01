use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use program1_contracts::{AuthContract, UserContract};
use program1_core::init_database;
use program1_module_analytics::AnalyticsModule;
use program1_module_audit::AuditModule;
use program1_module_auth::AuthModule;
use program1_module_catalog::CatalogModule;
use program1_module_channel::ChannelSyncModule;
use program1_module_inventory::InventoryModule;
use program1_module_order::OrderModule;
use program1_module_user::UserModule;
use program1_web::{create_app, rate_limit::IpRateLimiter, AppState};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn setup_test_app() -> (axum::Router, String) {
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
    let audit_module = Arc::new(AuditModule::new(pool.clone()));
    let rate_limiter = Arc::new(IpRateLimiter::new());

    let _ = user_module.seed_default_users().await;
    let _ = catalog_module.seed_default_catalog().await;
    let _ = channel_module.seed_default_channels().await;

    let admin_user = user_module.authenticate("admin", "admin123").await.unwrap();
    let admin_token = auth_module.generate_token(&admin_user).unwrap();

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
        rate_limiter,
        started_at: std::time::Instant::now(),
    };

    (create_app(state), admin_token)
}

#[tokio::test]
async fn test_inventory_list_and_detail_endpoints() {
    let (app, token) = setup_test_app().await;

    // GET /api/v1/inventory
    let req = Request::builder()
        .uri("/api/v1/inventory")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let stocks: Vec<Value> = serde_json::from_slice(&body).unwrap();
    assert!(!stocks.is_empty());

    let product_id = stocks[0]["product_id"].as_str().unwrap();

    // GET /api/v1/inventory/:id
    let req2 = Request::builder()
        .uri(format!("/api/v1/inventory/{}", product_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res2 = app.oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_update_all_stock_types_and_audit_logs() {
    let (app, token) = setup_test_app().await;

    // List to get product ID
    let req = Request::builder()
        .uri("/api/v1/inventory")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let stocks: Vec<Value> = serde_json::from_slice(&body).unwrap();
    let product_id = stocks[0]["product_id"].as_str().unwrap();

    // 1. POST /api/v1/inventory/:id/warehouse-stock
    let req_wh = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/inventory/{}/warehouse-stock", product_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({
            "new_warehouse_stock": 2000,
            "admin_note": "Restock kontainer dari supplier",
            "updated_by": "Admin Warehouse"
        }).to_string()))
        .unwrap();
    let res_wh = app.clone().oneshot(req_wh).await.unwrap();
    assert_eq!(res_wh.status(), StatusCode::OK);

    // 2. POST /api/v1/inventory/:id/safety-stock
    let req_safety = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/inventory/{}/safety-stock", product_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({
            "new_safety_stock": 100,
            "admin_note": "Safety stock 100 unit",
            "updated_by": "Admin"
        }).to_string()))
        .unwrap();
    let res_safety = app.clone().oneshot(req_safety).await.unwrap();
    assert_eq!(res_safety.status(), StatusCode::OK);

    // 3. POST /api/v1/inventory/:id/spare-stock
    let req_spare = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/inventory/{}/spare-stock", product_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({
            "new_spare_stock": 50,
            "admin_note": "Alokasi cadangan untuk giveaway",
            "updated_by": "Admin Marketing"
        }).to_string()))
        .unwrap();
    let res_spare = app.clone().oneshot(req_spare).await.unwrap();
    assert_eq!(res_spare.status(), StatusCode::OK);

    // 4. POST /api/v1/inventory/:id/promotion-stock
    let req_promo = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/inventory/{}/promotion-stock", product_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({
            "new_promotion_stock": 200,
            "admin_note": "Alokasi Flash Sale Shopee",
            "updated_by": "Admin Promo"
        }).to_string()))
        .unwrap();
    let res_promo = app.clone().oneshot(req_promo).await.unwrap();
    assert_eq!(res_promo.status(), StatusCode::OK);

    // 5. GET /api/v1/inventory/:id/adjustment-logs
    let req_logs = Request::builder()
        .uri(format!("/api/v1/inventory/{}/adjustment-logs", product_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let res_logs = app.clone().oneshot(req_logs).await.unwrap();
    assert_eq!(res_logs.status(), StatusCode::OK);
    let body_logs = to_bytes(res_logs.into_body(), usize::MAX).await.unwrap();
    let logs: Vec<Value> = serde_json::from_slice(&body_logs).unwrap();
    assert_eq!(logs.len(), 4);
}

#[tokio::test]
async fn test_bulk_update_and_low_stock_alerts() {
    let (app, token) = setup_test_app().await;

    // List products
    let req = Request::builder()
        .uri("/api/v1/inventory")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let stocks: Vec<Value> = serde_json::from_slice(&body).unwrap();
    let p1 = stocks[0]["product_id"].as_str().unwrap();
    let p2 = stocks[1]["product_id"].as_str().unwrap();

    // Bulk update: set p1 safety stock very high to trigger low stock alert
    let req_bulk = Request::builder()
        .method("POST")
        .uri("/api/v1/inventory/bulk-update")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({
            "adjustments": [
                {
                    "product_id": p1,
                    "stock_type": "safety",
                    "new_value": 700
                },
                {
                    "product_id": p2,
                    "stock_type": "spare",
                    "new_value": 20
                }
            ],
            "admin_note": "Penyesuaian stok massal akhir bulan",
            "updated_by": "Admin Pusat"
        }).to_string()))
        .unwrap();

    let res_bulk = app.clone().oneshot(req_bulk).await.unwrap();
    assert_eq!(res_bulk.status(), StatusCode::OK);

    let body_bulk = to_bytes(res_bulk.into_body(), usize::MAX).await.unwrap();
    let bulk_res: Value = serde_json::from_slice(&body_bulk).unwrap();
    assert_eq!(bulk_res["total_requested"], 2);
    assert_eq!(bulk_res["total_success"], 2);
    assert_eq!(bulk_res["total_failed"], 0);

    // GET /api/v1/inventory/alerts/low-stock
    let req_alert = Request::builder()
        .uri("/api/v1/inventory/alerts/low-stock")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res_alert = app.oneshot(req_alert).await.unwrap();
    assert_eq!(res_alert.status(), StatusCode::OK);

    let body_alert = to_bytes(res_alert.into_body(), usize::MAX).await.unwrap();
    let alerts: Vec<Value> = serde_json::from_slice(&body_alert).unwrap();
    assert!(!alerts.is_empty());
    assert!(alerts.iter().any(|a| a["product_id"] == p1));
}
