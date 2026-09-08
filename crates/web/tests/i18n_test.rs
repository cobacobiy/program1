use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
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

fn collect_keys(val: &Value, prefix: &str, keys: &mut BTreeSet<String>) {
    match val {
        Value::Object(map) => {
            for (k, v) in map {
                let full_key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                collect_keys(v, &full_key, keys);
            }
        }
        Value::Array(arr) => {
            for (idx, item) in arr.iter().enumerate() {
                let full_key = format!("{}[{}]", prefix, idx);
                collect_keys(item, &full_key, keys);
            }
        }
        _ => {
            keys.insert(prefix.to_string());
        }
    }
}

#[test]
fn test_i18n_json_validity_and_parity() {
    let id_path = Path::new("static/i18n/id.json");
    let en_path = Path::new("static/i18n/en.json");

    assert!(id_path.exists(), "id.json must exist at static/i18n/id.json");
    assert!(en_path.exists(), "en.json must exist at static/i18n/en.json");

    let id_raw = fs::read_to_string(id_path).expect("Failed to read id.json");
    let en_raw = fs::read_to_string(en_path).expect("Failed to read en.json");

    let id_val: Value = serde_json::from_str(&id_raw).expect("id.json must be valid JSON");
    let en_val: Value = serde_json::from_str(&en_raw).expect("en.json must be valid JSON");

    let mut id_keys = BTreeSet::new();
    let mut en_keys = BTreeSet::new();

    collect_keys(&id_val, "", &mut id_keys);
    collect_keys(&en_val, "", &mut en_keys);

    assert!(
        !id_keys.is_empty(),
        "id.json must contain translation entries"
    );
    assert!(
        !en_keys.is_empty(),
        "en.json must contain translation entries"
    );

    // Assert 100% symmetric key parity
    let missing_in_en: Vec<_> = id_keys.difference(&en_keys).collect();
    let missing_in_id: Vec<_> = en_keys.difference(&id_keys).collect();

    assert!(
        missing_in_en.is_empty(),
        "Keys missing in en.json: {:?}",
        missing_in_en
    );
    assert!(
        missing_in_id.is_empty(),
        "Keys missing in id.json: {:?}",
        missing_in_id
    );

    // Verify critical namespaces exist
    let critical_sections = [
        "nav", "catalog", "product", "cart", "checkout", "order", "wishlist", "auth", "admin",
        "common",
    ];
    for section in &critical_sections {
        assert!(
            id_val.get(section).is_some(),
            "Section '{}' must exist in id.json",
            section
        );
        assert!(
            en_val.get(section).is_some(),
            "Section '{}' must exist in en.json",
            section
        );
    }
}

#[test]
fn test_i18n_frontend_dictionary_sync() {
    let static_id =
        fs::read_to_string("static/i18n/id.json").expect("Failed to read static/i18n/id.json");
    let static_en =
        fs::read_to_string("static/i18n/en.json").expect("Failed to read static/i18n/en.json");

    let frontend_id_path = Path::new("../../frontend/src/lib/i18n/id.json");
    let frontend_en_path = Path::new("../../frontend/src/lib/i18n/en.json");

    if frontend_id_path.exists() {
        let frontend_id = fs::read_to_string(frontend_id_path).unwrap();
        let val1: Value = serde_json::from_str(&static_id).unwrap();
        let val2: Value = serde_json::from_str(&frontend_id).unwrap();
        assert_eq!(val1, val2, "Frontend id.json must match static id.json");
    }

    if frontend_en_path.exists() {
        let frontend_en = fs::read_to_string(frontend_en_path).unwrap();
        let val1: Value = serde_json::from_str(&static_en).unwrap();
        let val2: Value = serde_json::from_str(&frontend_en).unwrap();
        assert_eq!(val1, val2, "Frontend en.json must match static en.json");
    }
}

async fn setup_test_app() -> axum::Router {
    let secret = "test-jwt-secret-key-minimum-32-characters-length!".to_string();
    let pool = init_database("sqlite::memory:")
        .await
        .expect("Test DB init failed");

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
        auth_contract: auth_module,
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
        rate_limiter: Arc::new(program1_web::rate_limit::IpRateLimiter::new()),
        started_at: std::time::Instant::now(),
        google_client_id: "test".to_string(),
    };

    create_app(state)
}

#[tokio::test]
async fn test_i18n_static_assets_serving() {
    let app = setup_test_app().await;

    // Test GET /assets/i18n/id.json
    let req = Request::builder()
        .uri("/assets/i18n/id.json")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&body).expect("Must return valid json for id.json");
    assert!(val.get("nav").is_some());

    // Test GET /assets/i18n/en.json
    let req = Request::builder()
        .uri("/assets/i18n/en.json")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let val: Value = serde_json::from_slice(&body).expect("Must return valid json for en.json");
    assert!(val.get("nav").is_some());

    // Test GET /assets/store/store-i18n.js
    let req = Request::builder()
        .uri("/assets/store/store-i18n.js")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
