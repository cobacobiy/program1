use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use program1_core::init_tracing;
use program1_module_analytics::AnalyticsModule;
use program1_module_auth::AuthModule;
use program1_module_catalog::CatalogModule;
use program1_module_channel::ChannelSyncModule;
use program1_module_inventory::InventoryModule;
use program1_module_order::OrderModule;
use program1_module_user::UserModule;
use program1_web::{create_app, AppState};

#[tokio::main]
async fn main() {
    init_tracing();

    let store_name = env::var("STORE_NAME").unwrap_or_else(|_| "AURA Storefront".to_string());
    let store_currency = env::var("STORE_CURRENCY").unwrap_or_else(|_| "IDR".to_string());
    let jwt_secret = env::var("JWT_SECRET")
        .unwrap_or_else(|_| "super-secret-program1-jwt-signing-key-32chars-min!".to_string());
    let jwt_expiry_hours: u64 = env::var("JWT_EXPIRY_HOURS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(24);

    // 1. Instantiate domain modules
    let user_module = Arc::new(UserModule::new());
    let auth_module = Arc::new(AuthModule::new(jwt_secret, jwt_expiry_hours));
    let catalog_module = Arc::new(CatalogModule::new());
    let inventory_module = Arc::new(InventoryModule::new(catalog_module.clone()));
    let channel_module = Arc::new(ChannelSyncModule::new());
    let order_module = Arc::new(OrderModule::new(
        catalog_module.clone(),
        inventory_module.clone(),
    ));
    let analytics_module = Arc::new(AnalyticsModule::new(
        catalog_module.clone(),
        order_module.clone(),
    ));

    let state = AppState {
        store_name,
        store_currency,
        user_contract: user_module,
        auth_contract: auth_module,
        catalog_contract: catalog_module,
        inventory_contract: inventory_module,
        channel_contract: channel_module,
        order_contract: order_module,
        analytics_contract: analytics_module,
    };

    let app = create_app(state);

    let port: u16 = env::var("APP_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .unwrap_or(8080);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Starting Program1 Omnichannel Engine on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
