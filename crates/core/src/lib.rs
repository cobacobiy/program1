pub mod auth;
pub mod config;
pub mod database;
pub mod email;
pub mod sanitize;
pub use auth::{hash_password, verify_password};
pub use config::AppConfig;
pub use database::{init_database, DbPool};
pub use email::{
    order_confirmation_email, payment_success_email, shipping_notification_email, welcome_email,
    ConsoleEmailSender, EmailSender, MockEmailRecord, MockEmailSender, OrderEmailItem,
    SmtpEmailConfig, SmtpEmailSender,
};
pub use sanitize::{sanitize_text, strip_html};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,program1=debug,tower_http=debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}
