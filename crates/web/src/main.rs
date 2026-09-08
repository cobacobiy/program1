use std::net::SocketAddr;
use std::sync::Arc;

use program1_core::{init_database, init_tracing, AppConfig};
use program1_module_analytics::AnalyticsModule;
use program1_module_audit::AuditModule;
use program1_module_auth::AuthModule;
use program1_module_catalog::CatalogModule;
use program1_module_channel::ChannelSyncModule;
use program1_module_chat::ChatModule;
use program1_module_coupon::CouponModule;
use program1_module_inventory::InventoryModule;
use program1_module_order::OrderModule;
use program1_module_payment::PaymentModule;
use program1_module_review::ReviewModule;
use program1_module_shipping::ShippingModule;
use program1_module_return::ReturnModule;
use program1_module_user::UserModule;
use program1_web::{create_app, AppState};

#[tokio::main]
async fn main() {
    init_tracing();

    let config = AppConfig::from_env();

    // Production safety checks
    if config.is_production()
        && (config.jwt_secret.contains("super-secret") || config.jwt_secret.contains("CHANGE_ME"))
    {
        tracing::error!(
            "CRITICAL: In production environment, JWT_SECRET must be set to a secure, unique 32+ character key!"
        );
        std::process::exit(1);
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
    let user_module = Arc::new(
        UserModule::new(db_pool.clone())
            .with_dev_support_password(config.dev_support_password.clone()),
    );
    let auth_module = Arc::new(AuthModule::new(
        config.jwt_secret.clone(),
        config.jwt_expiry_hours,
    ));
    let catalog_module = Arc::new(CatalogModule::new(db_pool.clone()));
    let inventory_module = Arc::new(InventoryModule::new(
        db_pool.clone(),
        catalog_module.clone(),
    ));
    let channel_module = Arc::new(ChannelSyncModule::new(db_pool.clone()));
    let email_sender: Arc<dyn program1_core::EmailSender> =
        if config.email_provider.eq_ignore_ascii_case("smtp") {
            tracing::info!(
                host = %config.smtp_host,
                port = %config.smtp_port,
                user = %config.smtp_username,
                "Initializing SmtpEmailSender"
            );
            Arc::new(program1_core::SmtpEmailSender::new(
                program1_core::SmtpEmailConfig {
                    host: config.smtp_host.clone(),
                    port: config.smtp_port,
                    username: config.smtp_username.clone(),
                    password: config.smtp_password.clone(),
                    from_name: config.smtp_from_name.clone(),
                    from_email: config.smtp_from_email.clone(),
                },
            ))
        } else {
            tracing::info!("Initializing ConsoleEmailSender for development/test environment");
            Arc::new(program1_core::ConsoleEmailSender::new())
        };

    let notification_module = Arc::new(program1_module_notification::NotificationModule::new(
        db_pool.clone(),
    ));

    let order_module = Arc::new(
        OrderModule::new(
            db_pool.clone(),
            catalog_module.clone(),
            inventory_module.clone(),
        )
        .with_email_sender(email_sender.clone(), config.store_name.clone())
        .with_notification_contract(notification_module.clone()),
    );
    let analytics_module = Arc::new(AnalyticsModule::new(
        catalog_module.clone(),
        order_module.clone(),
    ));
    let audit_module = Arc::new(AuditModule::new(db_pool.clone()));

    let google_verifier = Arc::new(program1_module_buyer::ProductionGoogleVerifier {
        client_id: config.google_client_id.clone(),
    });
    let sms_sender = Arc::new(program1_module_buyer::ConsoleOrProviderSmsSender::new(
        config.app_env.clone(),
        config.sms_provider.clone(),
        config.sms_provider_api_key.clone(),
    ));
    let buyer_config = program1_module_buyer::BuyerModuleConfig {
        otp_expiry_seconds: config.otp_expiry_seconds as u64,
        otp_max_attempts: config.otp_max_attempts as u32,
        otp_resend_cooldown_seconds: config.otp_resend_cooldown_seconds as u64,
    };
    let buyer_module = Arc::new(
        program1_module_buyer::BuyerModule::new_with_config(
            db_pool.clone(),
            auth_module.clone(),
            google_verifier,
            sms_sender,
            audit_module.clone(),
            buyer_config,
        )
        .with_email_sender(email_sender.clone(), config.store_name.clone()),
    );
    let chat_module = Arc::new(ChatModule::new(db_pool.clone()));
    let payment_module = Arc::new(PaymentModule::new(
        db_pool.clone(),
        config.midtrans_server_key.clone(),
        config.midtrans_client_key.clone(),
        config.midtrans_is_production,
    ));
    let coupon_module = Arc::new(CouponModule::new(db_pool.clone()));
    let review_module = Arc::new(ReviewModule::new(db_pool.clone()));
    let shipping_module = Arc::new(ShippingModule::new(
        config.rajaongkir_api_key.clone(),
        config.rajaongkir_type.clone(),
        config.shipping_origin_city_id.clone(),
    ));
    let return_module = Arc::new(
        ReturnModule::new(db_pool.clone())
            .with_inventory_contract(inventory_module.clone())
            .with_notification_contract(notification_module.clone())
            .with_audit_contract(audit_module.clone()),
    );

    // Ensure initial seed runs
    let _ = user_module.seed_default_users().await;
    let _ = catalog_module.seed_default_catalog().await;
    let _ = channel_module.seed_default_channels().await;

    // Ensure uploads directory exists
    let _ = std::fs::create_dir_all("data/uploads");

    let state = AppState {
        store_name: config.store_name.clone(),
        store_currency: config.store_currency.clone(),
        store_whatsapp_number: config.store_whatsapp_number.clone(),
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
        shipping_contract: shipping_module,
        notification_contract: notification_module,
        return_contract: return_module,
        rate_limiter: Arc::new(program1_web::rate_limit::IpRateLimiter::new()),
        started_at: std::time::Instant::now(),
        google_client_id: config.google_client_id.clone(),
    };

    let app = create_app(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.app_port));
    tracing::info!("Starting Program1 Omnichannel Engine on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
