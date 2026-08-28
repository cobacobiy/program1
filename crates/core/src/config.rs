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
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            app_env: env_or("APP_ENV", "development"),
            app_port: env_or("APP_PORT", "8080").parse().unwrap_or(8080),
            store_name: env_or("STORE_NAME", "AURA Storefront"),
            store_currency: env_or("STORE_CURRENCY", "IDR"),
            database_url: env_or("DATABASE_URL", "sqlite://./data/program1.db?mode=rwc"),
            admin_default_password: env_or("ADMIN_DEFAULT_PASSWORD", "admin123"),
            jwt_secret: env_or(
                "JWT_SECRET",
                "super-secret-program1-jwt-signing-key-32chars-min!",
            ),
            jwt_expiry_hours: env_or("JWT_EXPIRY_HOURS", "24").parse().unwrap_or(24),
            allowed_origins: env_or("ALLOWED_ORIGINS", "http://localhost:8080,http://localhost:3000")
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
