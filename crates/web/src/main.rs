use std::net::SocketAddr;
use std::sync::Arc;

use program1_core::{init_database, init_tracing, AppConfig};
use program1_module_analytics::AnalyticsModule;
use program1_module_audit::AuditModule;
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

    let config = AppConfig::from_env();

    // Production safety checks
    if config.is_production() {
        if config.jwt_secret.contains("super-secret") || config.jwt_secret.contains("CHANGE_ME") {
            tracing::error!("CRITICAL: In production environment, JWT_SECRET must be set to a secure, unique 32+ character key!");
            std::process::exit(1);
        }
        if config.admin_default_password == "admin123" {
            tracing::warn!("SECURITY ALERT: Default admin password 'admin123' is active. Please update ADMIN_DEFAULT_PASSWORD!");
        }
    }

    tracing::info!(
        env = %config.app_env,
        port = %config.app_port,
        store = %config.store_name,
        "Program1 configuration initialized successfully"
    );

    // 1. Initialize persistent database pool & run migrations
    let db_pool = init_database(&config.database_url)
        .await
        .expect("Failed to initialize database and run migrations");

    // 2. Instantiate domain modules with database pool
    let user_module = Arc::new(UserModule::new(db_pool.clone()));
    let auth_module = Arc::new(AuthModule::new(config.jwt_secret.clone(), config.jwt_expiry_hours));
    let catalog_module = Arc::new(CatalogModule::new(db_pool.clone()));
    let inventory_module = Arc::new(InventoryModule::new(db_pool.clone(), catalog_module.clone()));
    let channel_module = Arc::new(ChannelSyncModule::new(db_pool.clone()));
    let order_module = Arc::new(OrderModule::new(
        db_pool.clone(),
        catalog_module.clone(),
        inventory_module.clone(),
    ));
    let analytics_module = Arc::new(AnalyticsModule::new(
        catalog_module.clone(),
        order_module.clone(),
    ));
    let audit_module = Arc::new(AuditModule::new(db_pool.clone()));

    // Ensure initial seed runs
    let _ = user_module.seed_default_users().await;
    let _ = catalog_module.seed_default_catalog().await;
    let _ = channel_module.seed_default_channels().await;

    let state = AppState {
        store_name: config.store_name.clone(),
        store_currency: config.store_currency.clone(),
        user_contract: user_module,
        auth_contract: auth_module,
        catalog_contract: catalog_module,
        inventory_contract: inventory_module,
        channel_contract: channel_module,
        order_contract: order_module,
        analytics_contract: analytics_module,
        audit_contract: audit_module,
        rate_limiter: Arc::new(program1_web::rate_limit::IpRateLimiter::new()),
    };

    let app = create_app(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.app_port));
    tracing::info!("Starting Program1 Omnichannel Engine on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
