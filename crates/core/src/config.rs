#[derive(Debug, Clone)]
pub struct AppConfig {
    pub app_env: String,
    pub app_port: u16,
    pub store_name: String,
    pub store_currency: String,
    pub database_url: String,
    pub admin_default_password: String,
    pub jwt_secret: String,
    pub jwt_expiry_hours: u64,
    pub allowed_origins: Vec<String>,
    pub rate_limit_per_second: u64,
    pub login_rate_limit_per_minute: u64,
    pub google_client_id: String,
    pub dev_support_password: Option<String>,
    pub sms_provider: String,
    pub sms_provider_api_key: String,
    pub otp_expiry_seconds: i64,
    pub otp_max_attempts: i64,
    pub otp_resend_cooldown_seconds: i64,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let app_env = env_or("APP_ENV", "development");
        let is_prod = app_env.eq_ignore_ascii_case("production");

        let jwt_secret = if is_prod {
            let secret = std::env::var("JWT_SECRET")
                .expect("CRITICAL: JWT_SECRET environment variable must be set in production!");
            if secret.len() < 32 {
                panic!("CRITICAL: JWT_SECRET in production must be at least 32 characters long!");
            }
            secret
        } else {
            env_or(
                "JWT_SECRET",
                "super-secret-program1-jwt-signing-key-32chars-min!",
            )
        };

        let admin_default_password = if is_prod {
            let pass = std::env::var("ADMIN_DEFAULT_PASSWORD")
                .expect("CRITICAL: ADMIN_DEFAULT_PASSWORD must be set in production!");
            if pass == "admin123" || pass.len() < 12 {
                panic!("CRITICAL: ADMIN_DEFAULT_PASSWORD is weak or uses default 'admin123' in production! Must be at least 12 characters.");
            }
            pass
        } else {
            env_or("ADMIN_DEFAULT_PASSWORD", "admin123")
        };

        let dev_support_password = if is_prod {
            let pass = std::env::var("DEV_SUPPORT_PASSWORD").expect(
                "CRITICAL: DEV_SUPPORT_PASSWORD must be explicitly configured in production!",
            );
            if pass == admin_default_password || pass == "admin123" || pass.len() < 16 {
                panic!("CRITICAL: DEV_SUPPORT_PASSWORD must be independent from ADMIN_DEFAULT_PASSWORD and at least 16 characters in production!");
            }
            Some(pass)
        } else {
            std::env::var("DEV_SUPPORT_PASSWORD").ok()
        };

        let google_client_id = env_or("GOOGLE_CLIENT_ID", "");
        let sms_provider = env_or("SMS_PROVIDER", "console");
        let sms_provider_api_key = env_or("SMS_PROVIDER_API_KEY", "");
        let otp_expiry_seconds = env_or("OTP_EXPIRY_SECONDS", "300")
            .parse::<i64>()
            .unwrap_or(300);
        let otp_max_attempts = env_or("OTP_MAX_ATTEMPTS", "3").parse::<i64>().unwrap_or(3);
        let otp_resend_cooldown_seconds = env_or("OTP_RESEND_COOLDOWN_SECONDS", "60")
            .parse::<i64>()
            .unwrap_or(60);

        Self {
            app_env,
            app_port: env_or("APP_PORT", "8080").parse().unwrap_or(8080),
            store_name: env_or("STORE_NAME", "AURA Storefront"),
            store_currency: env_or("STORE_CURRENCY", "IDR"),
            database_url: env_or("DATABASE_URL", "sqlite://./data/program1.db?mode=rwc"),
            admin_default_password,
            dev_support_password,
            jwt_secret,
            jwt_expiry_hours: env_or("JWT_EXPIRY_HOURS", "24").parse().unwrap_or(24),
            allowed_origins: env_or(
                "ALLOWED_ORIGINS",
                "http://localhost:8080,http://localhost:3000",
            )
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
            rate_limit_per_second: env_or("RATE_LIMIT_PER_SECOND", "100")
                .parse()
                .unwrap_or(100),
            login_rate_limit_per_minute: env_or("LOGIN_RATE_LIMIT_PER_MINUTE", "5")
                .parse()
                .unwrap_or(5),
            google_client_id,
            sms_provider,
            sms_provider_api_key,
            otp_expiry_seconds,
            otp_max_attempts,
            otp_resend_cooldown_seconds,
        }
    }

    pub fn is_production(&self) -> bool {
        self.app_env.eq_ignore_ascii_case("production")
    }

    pub fn is_staging(&self) -> bool {
        self.app_env.eq_ignore_ascii_case("staging")
    }

    pub fn is_development(&self) -> bool {
        !self.is_production() && !self.is_staging()
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::from_env();
        assert_eq!(config.store_currency, "IDR");
        assert!(!config.jwt_secret.is_empty());
        assert!(!config.allowed_origins.is_empty());
    }

    #[test]
    fn test_is_production() {
        let mut config = AppConfig::from_env();
        config.app_env = "production".to_string();
        assert!(config.is_production());
        assert!(!config.is_development());

        config.app_env = "development".to_string();
        assert!(!config.is_production());
        assert!(config.is_development());
    }

    #[test]
    fn test_origins_parsing() {
        let origins_raw = "http://localhost:8080, https://aura.example.com ";
        let parsed: Vec<String> = origins_raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0], "http://localhost:8080");
        assert_eq!(parsed[1], "https://aura.example.com");
    }
}
