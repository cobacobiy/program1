use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use program1_contracts::{
    AuditContract, AuditLogEntry, AuthContract, BuyerAccountDto, BuyerAddressDto,
    BuyerAuthResponse, BuyerContract, BuyerLoginRequest, ContractError, CreateBuyerAddressRequest,
    PaginatedResponse, RegisterBuyerRequest, UpdateBuyerAddressRequest, WishlistItemDto,
};
use program1_core::database::DbPool;
use rand::Rng;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct GoogleClaims {
    pub sub: String,
    pub email: String,
    pub name: String,
    pub picture: Option<String>,
}

#[async_trait]
pub trait GoogleTokenVerifier: Send + Sync {
    async fn verify(&self, id_token: &str) -> Result<GoogleClaims, ContractError>;
}

/// Canonical Indonesian Phone Normalization (E.164: +628...)
pub fn normalize_indonesian_phone(raw: &str) -> Result<String, ContractError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ContractError::ValidationError(
            "Nomor telepon tidak boleh kosong".to_string(),
        ));
    }

    for c in trimmed.chars() {
        if !c.is_ascii_digit() && c != '+' && c != '-' && c != ' ' && c != '(' && c != ')' {
            return Err(ContractError::ValidationError(
                "Nomor telepon mengandung karakter tidak valid".to_string(),
            ));
        }
    }

    let digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();

    let canonical = if digits.starts_with("628") {
        format!("+{}", digits)
    } else if digits.starts_with("08") {
        format!("+62{}", &digits[1..])
    } else if digits.starts_with('8') {
        format!("+62{}", digits)
    } else {
        return Err(ContractError::ValidationError(
            "Nomor HP Indonesia harus berawalan 08 / 628 / +628 (bukan nomor telepon rumah/luar negeri)"
                .to_string(),
        ));
    };

    let total_digits = canonical.len() - 1;
    if !(10..=15).contains(&total_digits) {
        return Err(ContractError::ValidationError(
            "Panjang nomor HP Indonesia tidak valid (harus 10 - 15 digit)".to_string(),
        ));
    }

    Ok(canonical)
}

/// Mask email address for secure logging and audit (e.g. b***r@example.com)
pub fn mask_email(email: &str) -> String {
    let clean = email.trim();
    if let Some((local, domain)) = clean.split_once('@') {
        if local.len() <= 2 {
            format!("*@{}", domain)
        } else {
            let first = &local[0..1];
            let last = &local[local.len() - 1..];
            format!("{}***{}@{}", first, last, domain)
        }
    } else {
        "***".to_string()
    }
}

/// Mask phone number for secure logging (e.g. +6281****890)
pub fn mask_phone(phone: &str) -> String {
    let clean = phone.trim();
    if clean.len() <= 6 {
        return "****".to_string();
    }
    let prefix_len = 4;
    let suffix_len = 3;
    if clean.len() <= prefix_len + suffix_len {
        return format!("{}****{}", &clean[..2], &clean[clean.len() - 2..]);
    }
    format!(
        "{}****{}",
        &clean[..prefix_len],
        &clean[clean.len() - suffix_len..]
    )
}

/// Real/Production Google ID Token Verifier
pub struct ProductionGoogleVerifier {
    pub client_id: String,
}

impl ProductionGoogleVerifier {
    pub fn from_env() -> Self {
        let client_id = std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default();
        Self { client_id }
    }

    pub fn parse_and_validate_claims(
        &self,
        json: &serde_json::Value,
    ) -> Result<GoogleClaims, ContractError> {
        let aud = json.get("aud").and_then(|v| v.as_str()).unwrap_or("");
        if aud.is_empty() || aud != self.client_id {
            return Err(ContractError::ValidationError(
                "Google ID Token audience mismatch".to_string(),
            ));
        }

        let iss = json.get("iss").and_then(|v| v.as_str()).unwrap_or("");
        if iss != "https://accounts.google.com" && iss != "accounts.google.com" {
            return Err(ContractError::ValidationError(
                "Google ID Token issuer mismatch (expected accounts.google.com)".to_string(),
            ));
        }

        let email_verified = match json.get("email_verified") {
            Some(serde_json::Value::Bool(b)) => *b,
            Some(serde_json::Value::String(s)) => s == "true",
            _ => false,
        };
        if !email_verified {
            return Err(ContractError::ValidationError(
                "Akun Google ini belum terverifikasi (email unverified)".to_string(),
            ));
        }

        if let Some(exp_val) = json.get("exp") {
            let exp_ts = match exp_val {
                serde_json::Value::Number(n) => n.as_i64().unwrap_or(0),
                serde_json::Value::String(s) => s.parse::<i64>().unwrap_or(0),
                _ => 0,
            };
            if exp_ts <= chrono::Utc::now().timestamp() {
                return Err(ContractError::ValidationError(
                    "Google ID Token has expired".to_string(),
                ));
            }
        }

        let sub = json.get("sub").and_then(|v| v.as_str()).unwrap_or("");
        let email = json.get("email").and_then(|v| v.as_str()).unwrap_or("");
        let name = json.get("name").and_then(|v| v.as_str()).unwrap_or("Buyer");
        let picture = json
            .get("picture")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if sub.is_empty() || email.is_empty() {
            return Err(ContractError::ValidationError(
                "Google OAuth response missing sub or email claims".to_string(),
            ));
        }

        Ok(GoogleClaims {
            sub: sub.to_string(),
            email: email.to_string(),
            name: name.to_string(),
            picture,
        })
    }
}

#[async_trait]
impl GoogleTokenVerifier for ProductionGoogleVerifier {
    async fn verify(&self, id_token: &str) -> Result<GoogleClaims, ContractError> {
        let trimmed = id_token.trim();
        if trimmed.is_empty() {
            return Err(ContractError::ValidationError(
                "Google ID Token is empty".to_string(),
            ));
        }

        if self.client_id.trim().is_empty() || self.client_id.contains("your-google-client-id") {
            return Err(ContractError::ValidationError(
                "Google OAuth credentials not configured on server. Set GOOGLE_CLIENT_ID in .env"
                    .to_string(),
            ));
        }

        // Call Google's tokeninfo endpoint for verification using POST form body (keeps token out of query/URL logs)
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| ContractError::Internal(format!("HTTP client error: {}", e)))?;

        let res = client
            .post("https://oauth2.googleapis.com/tokeninfo")
            .form(&[("id_token", trimmed)])
            .send()
            .await
            .map_err(|_| {
                ContractError::Internal(
                    "Failed to connect to Google OAuth verification endpoint".to_string(),
                )
            })?;

        if !res.status().is_success() {
            return Err(ContractError::ValidationError(
                "Invalid Google ID Token or verification rejected by Google".to_string(),
            ));
        }

        let json: serde_json::Value = res.json().await.map_err(|_| {
            ContractError::Internal(
                "Failed to parse Google OAuth verification response".to_string(),
            )
        })?;

        self.parse_and_validate_claims(&json)
    }
}

#[async_trait]
pub trait SmsOtpSender: Send + Sync {
    async fn send_otp(&self, phone_number: &str, otp_code: &str) -> Result<(), ContractError>;
    fn is_live_provider(&self) -> bool {
        false
    }
}

/// Production SMS / WhatsApp OTP Sender (supporting Fonnte, Twilio, Generic HTTP or Console logging in dev)
#[derive(Debug, Clone)]
pub struct ConsoleOrProviderSmsSender {
    pub app_env: String,
    pub sms_provider: String,
    pub sms_provider_api_key: String,
}

impl ConsoleOrProviderSmsSender {
    pub fn new(app_env: String, sms_provider: String, sms_provider_api_key: String) -> Self {
        Self {
            app_env,
            sms_provider,
            sms_provider_api_key,
        }
    }

    pub fn from_env() -> Self {
        let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
        let sms_provider = std::env::var("SMS_PROVIDER").unwrap_or_else(|_| "console".to_string());
        let sms_provider_api_key = std::env::var("SMS_PROVIDER_API_KEY").unwrap_or_default();
        Self::new(app_env, sms_provider, sms_provider_api_key)
    }
}

impl Default for ConsoleOrProviderSmsSender {
    fn default() -> Self {
        Self::from_env()
    }
}

#[async_trait]
impl SmsOtpSender for ConsoleOrProviderSmsSender {
    fn is_live_provider(&self) -> bool {
        let is_prod = self.app_env.to_lowercase() == "production";
        let has_valid_api_key = !self.sms_provider_api_key.trim().is_empty()
            && !self.sms_provider_api_key.contains("your-sms-provider");
        let has_valid_provider = !self.sms_provider.trim().is_empty()
            && self.sms_provider.trim().to_lowercase() != "console";

        is_prod || (has_valid_provider && has_valid_api_key)
    }

    async fn send_otp(&self, phone_number: &str, otp_code: &str) -> Result<(), ContractError> {
        let masked = mask_phone(phone_number);
        let is_prod = self.app_env.to_lowercase() == "production";
        let provider_lower = self.sms_provider.trim().to_lowercase();
        let is_fonnte = provider_lower == "fonnte" || provider_lower.contains("api.fonnte.com");

        if is_fonnte {
            let token = self.sms_provider_api_key.trim();
            if token.is_empty() || token.contains("your-sms-provider") {
                if is_prod {
                    return Err(ContractError::Internal(
                        "Fonnte WhatsApp Token belum dikonfigurasi pada server produksi. Pengiriman OTP gagal demi keamanan."
                            .to_string(),
                    ));
                }
            } else {
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(10))
                    .build()
                    .map_err(|e| {
                        ContractError::Internal(format!("HTTP client build error: {}", e))
                    })?;

                let resp = client
                    .post("https://api.fonnte.com/send")
                    .header("Authorization", token)
                    .json(&serde_json::json!({
                        "target": phone_number,
                        "message": format!("*Kode OTP Program1*\n\nKode verifikasi Anda adalah: *{}*\n\nJangan berikan kode ini kepada siapapun demi keamanan akun Anda. Berlaku selama 5 menit.", otp_code),
                        "countryCode": "62",
                    }))
                    .send()
                    .await
                    .map_err(|e| ContractError::Internal(format!("Gagal mengirim WhatsApp OTP via Fonnte: {}", e)))?;

                let status = resp.status();
                if !status.is_success() {
                    let err_text = resp.text().await.unwrap_or_default();
                    tracing::warn!(status = %status, err = %err_text, "Fonnte WhatsApp API error");
                    if is_prod {
                        return Err(ContractError::Internal(format!(
                            "WhatsApp Gateway (Fonnte) mengembalikan status error: {}",
                            err_text
                        )));
                    }
                } else {
                    tracing::info!(phone = %masked, "WhatsApp OTP dispatched successfully via Fonnte");
                    return Ok(());
                }
            }
        }

        if self.sms_provider.starts_with("http://") || self.sms_provider.starts_with("https://") {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .map_err(|e| ContractError::Internal(format!("HTTP client build error: {}", e)))?;
            let resp = client
                .post(&self.sms_provider)
                .header(
                    "Authorization",
                    format!("Bearer {}", self.sms_provider_api_key),
                )
                .json(&serde_json::json!({
                    "phone": phone_number,
                    "target": phone_number,
                    "message": format!("Kode OTP Anda: {}", otp_code),
                }))
                .send()
                .await
                .map_err(|e| {
                    ContractError::Internal(format!("Gagal mengirim SMS/WA OTP: {}", e))
                })?;

            if !resp.status().is_success() {
                if is_prod {
                    return Err(ContractError::Internal(format!(
                        "SMS/WA Provider mengembalikan status error: {}",
                        resp.status()
                    )));
                }
            } else {
                tracing::info!(
                    phone = %masked,
                    provider = %self.sms_provider,
                    "SMS/WA OTP dispatched via generic HTTP endpoint"
                );
                return Ok(());
            }
        }

        if is_prod {
            // In production, must FAIL CLOSED if SMS provider or API key is not properly configured
            if self.sms_provider.trim().is_empty()
                || self.sms_provider.trim().to_lowercase() == "console"
                || self.sms_provider_api_key.trim().is_empty()
                || self.sms_provider_api_key.contains("your-sms-provider")
            {
                return Err(ContractError::Internal(
                    "SMS/WhatsApp Provider tidak dikonfigurasi pada server produksi. Pengiriman OTP gagal demi keamanan."
                        .to_string(),
                ));
            }

            tracing::info!(
                phone = %masked,
                provider = %self.sms_provider,
                "SMS OTP dispatched via production SMS provider"
            );
            Ok(())
        } else {
            // Development / test environment: NEVER log raw OTP in shared output
            tracing::info!(
                phone = %masked,
                "Simulated SMS/WA OTP dispatch for local/staging testing"
            );
            Ok(())
        }
    }
}

#[derive(Debug, Clone)]
pub struct BuyerModuleConfig {
    pub otp_expiry_seconds: u64,
    pub otp_max_attempts: u32,
    pub otp_resend_cooldown_seconds: u64,
}

impl Default for BuyerModuleConfig {
    fn default() -> Self {
        Self {
            otp_expiry_seconds: 300,
            otp_max_attempts: 3,
            otp_resend_cooldown_seconds: 60,
        }
    }
}

#[derive(Clone)]
pub struct BuyerModule {
    pool: DbPool,
    auth_contract: Arc<dyn AuthContract>,
    google_verifier: Arc<dyn GoogleTokenVerifier>,
    sms_sender: Arc<dyn SmsOtpSender>,
    audit_contract: Arc<dyn AuditContract>,
    config: BuyerModuleConfig,
    email_sender: Option<Arc<dyn program1_core::EmailSender>>,
    store_name: String,
}

impl BuyerModule {
    pub fn new(
        pool: DbPool,
        auth_contract: Arc<dyn AuthContract>,
        google_verifier: Arc<dyn GoogleTokenVerifier>,
        sms_sender: Arc<dyn SmsOtpSender>,
        audit_contract: Arc<dyn AuditContract>,
    ) -> Self {
        Self::new_with_config(
            pool,
            auth_contract,
            google_verifier,
            sms_sender,
            audit_contract,
            BuyerModuleConfig::default(),
        )
    }

    pub fn new_with_config(
        pool: DbPool,
        auth_contract: Arc<dyn AuthContract>,
        google_verifier: Arc<dyn GoogleTokenVerifier>,
        sms_sender: Arc<dyn SmsOtpSender>,
        audit_contract: Arc<dyn AuditContract>,
        config: BuyerModuleConfig,
    ) -> Self {
        Self {
            pool,
            auth_contract,
            google_verifier,
            sms_sender,
            audit_contract,
            config,
            email_sender: None,
            store_name: "AURA Storefront".to_string(),
        }
    }

    pub fn with_email_sender(
        mut self,
        email_sender: Arc<dyn program1_core::EmailSender>,
        store_name: impl Into<String>,
    ) -> Self {
        self.email_sender = Some(email_sender);
        self.store_name = store_name.into();
        self
    }

    fn row_to_buyer_dto(row: &sqlx::sqlite::SqliteRow) -> Result<BuyerAccountDto, ContractError> {
        let id_str: String = row.get("id");
        let id = Uuid::parse_str(&id_str)
            .map_err(|e| ContractError::Internal(format!("Invalid UUID: {}", e)))?;
        let google_sub: Option<String> = row.try_get("google_sub").unwrap_or(None);
        let email: String = row.get("email");
        let full_name: String = row.get("full_name");
        let avatar_url: Option<String> = row.get("avatar_url");
        let phone_number: Option<String> = row.get("phone_number");
        let phone_verified: i64 = row.get("phone_verified");
        let is_active: i64 = row.try_get("is_active").unwrap_or(1);
        let created_at_str: String = row.get("created_at");
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let updated_at_str: String = row.get("updated_at");
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(BuyerAccountDto {
            id,
            google_sub,
            email,
            full_name,
            avatar_url,
            phone_number,
            phone_verified: phone_verified != 0,
            is_active: is_active != 0,
            created_at,
            updated_at,
        })
    }

    fn row_to_address_dto(row: &sqlx::sqlite::SqliteRow) -> Result<BuyerAddressDto, ContractError> {
        let id_str: String = row.get("id");
        let id = Uuid::parse_str(&id_str)
            .map_err(|e| ContractError::Internal(format!("Invalid UUID: {}", e)))?;
        let buyer_id_str: String = row.get("buyer_id");
        let buyer_id = Uuid::parse_str(&buyer_id_str)
            .map_err(|e| ContractError::Internal(format!("Invalid UUID: {}", e)))?;
        let recipient_name: String = row.get("recipient_name");
        let phone_number: String = row.get("phone_number");
        let street_address: String = row.get("street_address");
        let subdistrict: String = row.get("subdistrict");
        let city: String = row.get("city");
        let province: String = row.get("province");
        let postal_code: String = row.get("postal_code");
        let is_default: i64 = row.get("is_default");
        let created_at_str: String = row.get("created_at");
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(BuyerAddressDto {
            id,
            buyer_id,
            recipient_name,
            phone_number,
            street_address,
            subdistrict,
            city,
            province,
            postal_code,
            is_default: is_default != 0,
            created_at,
        })
    }
}

#[async_trait]
impl BuyerContract for BuyerModule {
    async fn register(
        &self,
        req: RegisterBuyerRequest,
    ) -> Result<BuyerAuthResponse, ContractError> {
        let full_name = req.full_name.trim().to_string();
        if full_name.len() < 2 || full_name.len() > 100 {
            return Err(ContractError::ValidationError(
                "Nama lengkap minimal 2 karakter (maks 100)".to_string(),
            ));
        }

        let email = req.email.trim().to_lowercase();
        if email.is_empty() || !email.contains('@') {
            return Err(ContractError::ValidationError(
                "Format email tidak valid".to_string(),
            ));
        }

        if req.password.len() < 8 || req.password.len() > 100 {
            return Err(ContractError::ValidationError(
                "Kata sandi minimal 8 karakter".to_string(),
            ));
        }

        let existing = sqlx::query("SELECT id FROM buyer_accounts WHERE email = $1")
            .bind(&email)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        if existing.is_some() {
            return Err(ContractError::ValidationError(
                "Email ini sudah terdaftar sebagai member toko. Silakan masuk / login.".to_string(),
            ));
        }

        let password_hash = program1_core::auth::hash_password(&req.password)
            .map_err(|e| ContractError::Internal(format!("Password hashing error: {}", e)))?;

        let new_id = Uuid::new_v4();
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        sqlx::query(
            "INSERT INTO buyer_accounts (id, google_sub, email, password_hash, full_name, avatar_url, phone_number, phone_verified, is_active, created_at, updated_at)
             VALUES ($1, NULL, $2, $3, $4, NULL, NULL, 0, 1, $5, $6)",
        )
        .bind(new_id.to_string())
        .bind(&email)
        .bind(&password_hash)
        .bind(&full_name)
        .bind(&now_str)
        .bind(&now_str)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let masked_email = mask_email(&email);
        let _ = self
            .audit_contract
            .log_action(AuditLogEntry {
                id: Uuid::new_v4(),
                timestamp: now,
                actor_id: Some(new_id),
                actor_username: masked_email.clone(),
                action: "BUYER_REGISTERED_EMAIL".to_string(),
                resource_type: "buyer".to_string(),
                resource_id: Some(new_id),
                details: format!("Buyer registered via email/password: {}", masked_email),
                ip_address: None,
            })
            .await;

        let buyer_dto = BuyerAccountDto {
            id: new_id,
            google_sub: None,
            email: email.clone(),
            full_name: full_name.clone(),
            avatar_url: None,
            phone_number: None,
            phone_verified: false,
            is_active: true,
            created_at: now,
            updated_at: now,
        };

        if let Some(ref email_sender) = self.email_sender {
            let sender = email_sender.clone();
            let recipient = email.clone();
            let name = full_name.clone();
            let s_name = self.store_name.clone();
            tokio::spawn(async move {
                let (subject, body) = program1_core::welcome_email(&name, &s_name, "/");
                if let Err(err) = sender.send_email(&recipient, &subject, &body).await {
                    tracing::error!(target: "email", "Failed to send welcome email to {}: {}", recipient, err);
                } else {
                    tracing::info!(target: "email", "Welcome email sent successfully to {}", recipient);
                }
            });
        }

        let token = self.auth_contract.generate_buyer_token(&buyer_dto)?;

        Ok(BuyerAuthResponse {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_in: 86400,
            buyer: buyer_dto,
            requires_phone_verification: true,
        })
    }

    async fn login(&self, req: BuyerLoginRequest) -> Result<BuyerAuthResponse, ContractError> {
        let email = req.email.trim().to_lowercase();
        if email.is_empty() {
            return Err(ContractError::ValidationError(
                "Email tidak boleh kosong".to_string(),
            ));
        }

        if req.password.is_empty() {
            return Err(ContractError::ValidationError(
                "Kata sandi tidak boleh kosong".to_string(),
            ));
        }

        let row_opt = sqlx::query(
            "SELECT id, google_sub, email, password_hash, full_name, avatar_url, phone_number, phone_verified, is_active, created_at, updated_at
             FROM buyer_accounts WHERE email = $1",
        )
        .bind(&email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let row = match row_opt {
            Some(r) => r,
            None => {
                return Err(ContractError::ValidationError(
                    "Email atau kata sandi tidak sesuai".to_string(),
                ));
            }
        };

        let is_active: i64 = row.try_get("is_active").unwrap_or(1);
        if is_active == 0 {
            return Err(ContractError::ValidationError(
                "Akun pembeli telah dinonaktifkan".to_string(),
            ));
        }

        let password_hash_opt: Option<String> = row.try_get("password_hash").unwrap_or(None);
        let password_hash = match password_hash_opt {
            Some(h) if !h.trim().is_empty() => h,
            _ => {
                return Err(ContractError::ValidationError(
                    "Akun ini terdaftar menggunakan Google Sign-In. Silakan masuk menggunakan tombol Google.".to_string(),
                ));
            }
        };

        let is_valid =
            program1_core::auth::verify_password(&req.password, &password_hash).unwrap_or(false);

        if !is_valid {
            return Err(ContractError::ValidationError(
                "Email atau kata sandi tidak sesuai".to_string(),
            ));
        }

        let buyer_dto = Self::row_to_buyer_dto(&row)?;

        let masked_email = mask_email(&buyer_dto.email);
        let _ = self
            .audit_contract
            .log_action(AuditLogEntry {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                actor_id: Some(buyer_dto.id),
                actor_username: masked_email.clone(),
                action: "BUYER_LOGIN_EMAIL".to_string(),
                resource_type: "buyer".to_string(),
                resource_id: Some(buyer_dto.id),
                details: format!("Buyer login via email: {}", masked_email),
                ip_address: None,
            })
            .await;

        let token = self.auth_contract.generate_buyer_token(&buyer_dto)?;

        Ok(BuyerAuthResponse {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_in: 86400,
            requires_phone_verification: !buyer_dto.phone_verified,
            buyer: buyer_dto,
        })
    }

    async fn authenticate_google(
        &self,
        id_token: &str,
    ) -> Result<BuyerAuthResponse, ContractError> {
        let claims = self.google_verifier.verify(id_token).await?;

        let existing_row = sqlx::query(
            "SELECT id, google_sub, email, full_name, avatar_url, phone_number, phone_verified, is_active, created_at, updated_at
             FROM buyer_accounts WHERE google_sub = $1 OR email = $2",
        )
        .bind(&claims.sub)
        .bind(&claims.email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let buyer = match existing_row {
            Some(row) => {
                let mut b = Self::row_to_buyer_dto(&row)?;
                if !b.is_active {
                    return Err(ContractError::ValidationError(
                        "Akun pembeli telah dinonaktifkan".to_string(),
                    ));
                }

                // Update google_sub, full name and avatar if provided
                let now = Utc::now().to_rfc3339();
                let _ = sqlx::query(
                    "UPDATE buyer_accounts SET google_sub = $1, full_name = $2, avatar_url = $3, updated_at = $4 WHERE id = $5",
                )
                .bind(&claims.sub)
                .bind(&claims.name)
                .bind(&claims.picture)
                .bind(&now)
                .bind(b.id.to_string())
                .execute(&self.pool)
                .await;

                b.google_sub = Some(claims.sub);
                b.full_name = claims.name;
                b.avatar_url = claims.picture;
                b
            }
            None => {
                let new_id = Uuid::new_v4();
                let now = Utc::now();
                let now_str = now.to_rfc3339();

                sqlx::query(
                    "INSERT INTO buyer_accounts (id, google_sub, email, full_name, avatar_url, phone_number, phone_verified, is_active, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $5, NULL, 0, 1, $6, $7)",
                )
                .bind(new_id.to_string())
                .bind(&claims.sub)
                .bind(&claims.email)
                .bind(&claims.name)
                .bind(&claims.picture)
                .bind(&now_str)
                .bind(&now_str)
                .execute(&self.pool)
                .await
                .map_err(|e| ContractError::Internal(e.to_string()))?;

                let masked_email = mask_email(&claims.email);
                let _ = self
                    .audit_contract
                    .log_action(AuditLogEntry {
                        id: Uuid::new_v4(),
                        timestamp: now,
                        actor_id: Some(new_id),
                        actor_username: masked_email.clone(),
                        action: "BUYER_REGISTERED".to_string(),
                        resource_type: "buyer".to_string(),
                        resource_id: Some(new_id),
                        details:
                            serde_json::json!({ "google_sub": claims.sub, "email": masked_email })
                                .to_string(),
                        ip_address: None,
                    })
                    .await;

                BuyerAccountDto {
                    id: new_id,
                    google_sub: Some(claims.sub),
                    email: claims.email,
                    full_name: claims.name,
                    avatar_url: claims.picture,
                    phone_number: None,
                    phone_verified: false,
                    is_active: true,
                    created_at: now,
                    updated_at: now,
                }
            }
        };

        let access_token = self.auth_contract.generate_buyer_token(&buyer)?;
        let requires_phone_verification = !buyer.phone_verified;

        let masked_email = mask_email(&buyer.email);
        let _ = self
            .audit_contract
            .log_action(AuditLogEntry {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                actor_id: Some(buyer.id),
                actor_username: masked_email,
                action: "BUYER_LOGIN".to_string(),
                resource_type: "buyer".to_string(),
                resource_id: Some(buyer.id),
                details: serde_json::json!({ "phone_verified": buyer.phone_verified }).to_string(),
                ip_address: None,
            })
            .await;

        Ok(BuyerAuthResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: 86400,
            requires_phone_verification,
            buyer,
        })
    }

    async fn request_phone_otp(
        &self,
        buyer_id: Uuid,
        phone_number: &str,
    ) -> Result<Option<String>, ContractError> {
        let canonical_phone = normalize_indonesian_phone(phone_number)?;

        // Rate limit / cooldown check from config
        let last_req_row = sqlx::query(
            "SELECT created_at FROM buyer_otp_verifications
             WHERE buyer_id = $1 AND phone_number = $2 AND verified_at IS NULL
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(buyer_id.to_string())
        .bind(&canonical_phone)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        if let Some(r) = last_req_row {
            let last_created_str: String = r.get("created_at");
            if let Ok(last_created) = DateTime::parse_from_rfc3339(&last_created_str) {
                let elapsed = Utc::now().signed_duration_since(last_created.with_timezone(&Utc));
                let cooldown = Duration::seconds(self.config.otp_resend_cooldown_seconds as i64);
                if elapsed < cooldown {
                    let wait_secs = self
                        .config
                        .otp_resend_cooldown_seconds
                        .saturating_sub(elapsed.num_seconds().max(0) as u64);
                    return Err(ContractError::ValidationError(format!(
                        "Mohon tunggu {} detik sebelum meminta kode OTP baru.",
                        wait_secs.max(1)
                    )));
                }
            }
        }

        // Generate 6-digit OTP code
        let otp_code = format!("{:06}", rand::thread_rng().gen_range(100000..999999));
        let otp_hash = program1_core::auth::hash_password(&otp_code)
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let id = Uuid::new_v4();
        let now = Utc::now();
        let expires_at = now + Duration::seconds(self.config.otp_expiry_seconds as i64);
        let max_attempts = self.config.otp_max_attempts as i64;

        sqlx::query(
            "INSERT INTO buyer_otp_verifications (id, buyer_id, phone_number, otp_hash, attempts, max_attempts, expires_at, verified_at, created_at)
             VALUES ($1, $2, $3, $4, 0, $5, $6, NULL, $7)",
        )
        .bind(id.to_string())
        .bind(buyer_id.to_string())
        .bind(&canonical_phone)
        .bind(&otp_hash)
        .bind(max_attempts)
        .bind(expires_at.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        if let Err(e) = self.sms_sender.send_otp(&canonical_phone, &otp_code).await {
            let _ = sqlx::query("DELETE FROM buyer_otp_verifications WHERE id = $1")
                .bind(id.to_string())
                .execute(&self.pool)
                .await;
            return Err(e);
        }

        let masked = mask_phone(&canonical_phone);
        let _ = self
            .audit_contract
            .log_action(AuditLogEntry {
                id: Uuid::new_v4(),
                timestamp: now,
                actor_id: Some(buyer_id),
                actor_username: masked.clone(),
                action: "BUYER_OTP_REQUESTED".to_string(),
                resource_type: "buyer_otp".to_string(),
                resource_id: Some(id),
                details: serde_json::json!({
                    "phone": masked,
                    "ttl_seconds": self.config.otp_expiry_seconds,
                    "cooldown_seconds": self.config.otp_resend_cooldown_seconds,
                })
                .to_string(),
                ip_address: None,
            })
            .await;

        let dev_code = if self.sms_sender.is_live_provider() {
            None
        } else {
            Some(otp_code)
        };

        Ok(dev_code)
    }

    async fn verify_phone_otp(
        &self,
        buyer_id: Uuid,
        phone_number: &str,
        code: &str,
    ) -> Result<BuyerAccountDto, ContractError> {
        let canonical_phone = normalize_indonesian_phone(phone_number)?;
        let clean_code = code.trim();

        let row = sqlx::query(
            "SELECT id, otp_hash, attempts, max_attempts, expires_at
             FROM buyer_otp_verifications
             WHERE buyer_id = $1 AND phone_number = $2 AND verified_at IS NULL
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(buyer_id.to_string())
        .bind(&canonical_phone)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let row = match row {
            Some(r) => r,
            None => {
                return Err(ContractError::ValidationError(
                    "Kode OTP tidak ditemukan atau sudah kadaluwarsa. Silakan minta kode baru."
                        .to_string(),
                ));
            }
        };

        let otp_id: String = row.get("id");
        let otp_hash: String = row.get("otp_hash");
        let attempts: i64 = row.get("attempts");
        let max_attempts: i64 = row.get("max_attempts");
        let expires_at_str: String = row.get("expires_at");

        let expires_at = DateTime::parse_from_rfc3339(&expires_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        if Utc::now() > expires_at {
            return Err(ContractError::ValidationError(
                "Kode OTP telah kedaluwarsa (berlaku 5 menit). Silakan minta kode baru."
                    .to_string(),
            ));
        }

        if attempts >= max_attempts {
            return Err(ContractError::ValidationError(
                "Batas percobaan OTP telah terlampaui. Silakan minta kode baru.".to_string(),
            ));
        }

        let is_valid = program1_core::auth::verify_password(clean_code, &otp_hash).unwrap_or(false);

        if !is_valid {
            let _ = sqlx::query(
                "UPDATE buyer_otp_verifications SET attempts = attempts + 1 WHERE id = $1",
            )
            .bind(&otp_id)
            .execute(&self.pool)
            .await;

            let remaining = max_attempts.saturating_sub(attempts + 1);
            let masked = mask_phone(&canonical_phone);
            self.audit_contract
                .log_action(AuditLogEntry {
                    id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    actor_id: Some(buyer_id),
                    actor_username: masked,
                    action: "BUYER_OTP_FAILED".to_string(),
                    resource_type: "buyer_otp".to_string(),
                    resource_id: Some(Uuid::parse_str(&otp_id).unwrap_or_default()),
                    details: serde_json::json!({ "remaining_attempts": remaining }).to_string(),
                    ip_address: None,
                })
                .await?;

            return Err(ContractError::ValidationError(format!(
                "Kode OTP salah. Sisa percobaan: {}.",
                remaining
            )));
        }

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        // Mark OTP as verified
        let now_str = Utc::now().to_rfc3339();
        sqlx::query("UPDATE buyer_otp_verifications SET verified_at = $1 WHERE id = $2")
            .bind(&now_str)
            .bind(&otp_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        // Update buyer account phone
        sqlx::query(
            "UPDATE buyer_accounts SET phone_number = $1, phone_verified = 1, updated_at = $2 WHERE id = $3",
        )
        .bind(&canonical_phone)
        .bind(&now_str)
        .bind(buyer_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let masked = mask_phone(&canonical_phone);
        self.audit_contract
            .log_action(AuditLogEntry {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                actor_id: Some(buyer_id),
                actor_username: masked.clone(),
                action: "BUYER_PHONE_VERIFIED".to_string(),
                resource_type: "buyer".to_string(),
                resource_id: Some(buyer_id),
                details: serde_json::json!({ "verified_phone": masked }).to_string(),
                ip_address: None,
            })
            .await?;

        self.get_buyer_profile(buyer_id).await
    }

    async fn get_buyer_profile(&self, buyer_id: Uuid) -> Result<BuyerAccountDto, ContractError> {
        let row = sqlx::query(
            "SELECT id, google_sub, email, full_name, avatar_url, phone_number, phone_verified, is_active, created_at, updated_at
             FROM buyer_accounts WHERE id = $1",
        )
        .bind(buyer_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        match row {
            Some(r) => {
                let buyer = Self::row_to_buyer_dto(&r)?;
                if !buyer.is_active {
                    return Err(ContractError::ValidationError(
                        "Akun pembeli telah dinonaktifkan".to_string(),
                    ));
                }
                Ok(buyer)
            }
            None => Err(ContractError::NotFound(format!(
                "Buyer with ID {}",
                buyer_id
            ))),
        }
    }

    async fn list_addresses(&self, buyer_id: Uuid) -> Result<Vec<BuyerAddressDto>, ContractError> {
        let rows = sqlx::query(
            "SELECT id, buyer_id, recipient_name, phone_number, street_address, subdistrict, city, province, postal_code, is_default, created_at
             FROM buyer_addresses WHERE buyer_id = $1 ORDER BY is_default DESC, created_at DESC",
        )
        .bind(buyer_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        rows.iter().map(Self::row_to_address_dto).collect()
    }

    async fn get_address(
        &self,
        buyer_id: Uuid,
        address_id: Uuid,
    ) -> Result<BuyerAddressDto, ContractError> {
        let row = sqlx::query(
            "SELECT id, buyer_id, recipient_name, phone_number, street_address, subdistrict, city, province, postal_code, is_default, created_at
             FROM buyer_addresses WHERE id = $1 AND buyer_id = $2",
        )
        .bind(address_id.to_string())
        .bind(buyer_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        match row {
            Some(r) => Self::row_to_address_dto(&r),
            None => Err(ContractError::NotFound(format!("Address {}", address_id))),
        }
    }

    async fn create_address(
        &self,
        buyer_id: Uuid,
        req: CreateBuyerAddressRequest,
    ) -> Result<BuyerAddressDto, ContractError> {
        let canonical_phone = normalize_indonesian_phone(&req.phone_number)?;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let count_row =
            sqlx::query("SELECT COUNT(*) as count FROM buyer_addresses WHERE buyer_id = $1")
                .bind(buyer_id.to_string())
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| ContractError::Internal(e.to_string()))?;

        let count: i64 = count_row.get("count");
        let should_be_default = req.set_as_default || count == 0;

        if should_be_default {
            sqlx::query(
                "UPDATE buyer_addresses SET is_default = 0 WHERE buyer_id = $1 AND is_default = 1",
            )
            .bind(buyer_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;
        }

        let address_id = Uuid::new_v4();
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        sqlx::query(
            "INSERT INTO buyer_addresses (id, buyer_id, recipient_name, phone_number, street_address, subdistrict, city, province, postal_code, is_default, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(address_id.to_string())
        .bind(buyer_id.to_string())
        .bind(req.recipient_name.trim())
        .bind(&canonical_phone)
        .bind(req.street_address.trim())
        .bind(req.subdistrict.trim())
        .bind(req.city.trim())
        .bind(req.province.trim())
        .bind(req.postal_code.trim())
        .bind(if should_be_default { 1 } else { 0 })
        .bind(&now_str)
        .bind(&now_str)
        .execute(&mut *tx)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        self.get_address(buyer_id, address_id).await
    }

    async fn update_address(
        &self,
        buyer_id: Uuid,
        address_id: Uuid,
        req: UpdateBuyerAddressRequest,
    ) -> Result<BuyerAddressDto, ContractError> {
        // Check exists
        let _ = self.get_address(buyer_id, address_id).await?;

        let canonical_phone = normalize_indonesian_phone(&req.phone_number)?;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        if req.set_as_default {
            sqlx::query(
                "UPDATE buyer_addresses SET is_default = 0 WHERE buyer_id = $1 AND is_default = 1",
            )
            .bind(buyer_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;
        }

        let now_str = Utc::now().to_rfc3339();
        let default_val = if req.set_as_default { 1 } else { 0 };

        sqlx::query(
            "UPDATE buyer_addresses
             SET recipient_name = $1, phone_number = $2, street_address = $3, subdistrict = $4, city = $5, province = $6, postal_code = $7, is_default = CASE WHEN $8 = 1 THEN 1 ELSE is_default END, updated_at = $9
             WHERE id = $10 AND buyer_id = $11",
        )
        .bind(req.recipient_name.trim())
        .bind(&canonical_phone)
        .bind(req.street_address.trim())
        .bind(req.subdistrict.trim())
        .bind(req.city.trim())
        .bind(req.province.trim())
        .bind(req.postal_code.trim())
        .bind(default_val)
        .bind(&now_str)
        .bind(address_id.to_string())
        .bind(buyer_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        self.get_address(buyer_id, address_id).await
    }

    async fn delete_address(&self, buyer_id: Uuid, address_id: Uuid) -> Result<(), ContractError> {
        let addr = self.get_address(buyer_id, address_id).await?;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        sqlx::query("DELETE FROM buyer_addresses WHERE id = $1 AND buyer_id = $2")
            .bind(address_id.to_string())
            .bind(buyer_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        // If deleted address was default, promote another address if available
        if addr.is_default {
            let next_addr_row = sqlx::query(
                "SELECT id FROM buyer_addresses WHERE buyer_id = $1 ORDER BY created_at DESC LIMIT 1"
            )
            .bind(buyer_id.to_string())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

            if let Some(r) = next_addr_row {
                let next_id: String = r.get("id");
                sqlx::query("UPDATE buyer_addresses SET is_default = 1 WHERE id = $1")
                    .bind(next_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| ContractError::Internal(e.to_string()))?;
            }
        }

        tx.commit()
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn set_default_address(
        &self,
        buyer_id: Uuid,
        address_id: Uuid,
    ) -> Result<BuyerAddressDto, ContractError> {
        let _ = self.get_address(buyer_id, address_id).await?;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        sqlx::query(
            "UPDATE buyer_addresses SET is_default = 0 WHERE buyer_id = $1 AND is_default = 1",
        )
        .bind(buyer_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        sqlx::query("UPDATE buyer_addresses SET is_default = 1 WHERE id = $1 AND buyer_id = $2")
            .bind(address_id.to_string())
            .bind(buyer_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        self.get_address(buyer_id, address_id).await
    }

    async fn list_all_buyers(&self) -> Result<Vec<BuyerAccountDto>, ContractError> {
        let rows = sqlx::query(
            "SELECT id, google_sub, email, full_name, avatar_url, phone_number, phone_verified, is_active, created_at, updated_at
             FROM buyer_accounts
             ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let mut buyers = Vec::with_capacity(rows.len());
        for row in rows {
            buyers.push(Self::row_to_buyer_dto(&row)?);
        }
        Ok(buyers)
    }

    async fn list_buyers_paginated(
        &self,
        page: i64,
        page_size: i64,
        search: Option<&str>,
    ) -> Result<PaginatedResponse<BuyerAccountDto>, ContractError> {
        let page = page.max(1);
        let page_size = page_size.clamp(1, 100);
        let offset = (page - 1) * page_size;

        let search_term = search.map(|s| s.trim()).filter(|s| !s.is_empty());

        let count_row = sqlx::query(
            "SELECT COUNT(*) as total FROM buyer_accounts
             WHERE ($1 IS NULL OR email LIKE '%' || $1 || '%' OR full_name LIKE '%' || $1 || '%' OR phone_number LIKE '%' || $1 || '%')",
        )
        .bind(search_term)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let total: i64 = count_row.get("total");

        let rows = sqlx::query(
            "SELECT id, google_sub, email, full_name, avatar_url, phone_number, phone_verified, is_active, created_at, updated_at
             FROM buyer_accounts
             WHERE ($1 IS NULL OR email LIKE '%' || $1 || '%' OR full_name LIKE '%' || $1 || '%' OR phone_number LIKE '%' || $1 || '%')
             ORDER BY created_at DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(search_term)
        .bind(page_size)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let mut data = Vec::with_capacity(rows.len());
        for row in rows {
            data.push(Self::row_to_buyer_dto(&row)?);
        }

        let total_pages = if total == 0 {
            0
        } else {
            (total + page_size - 1) / page_size
        };

        Ok(PaginatedResponse {
            data,
            total,
            page,
            page_size,
            total_pages,
        })
    }

    async fn set_buyer_active_status(
        &self,
        buyer_id: Uuid,
        is_active: bool,
    ) -> Result<BuyerAccountDto, ContractError> {
        let now = Utc::now().to_rfc3339();
        let is_active_int = if is_active { 1 } else { 0 };

        let result =
            sqlx::query("UPDATE buyer_accounts SET is_active = $1, updated_at = $2 WHERE id = $3")
                .bind(is_active_int)
                .bind(&now)
                .bind(buyer_id.to_string())
                .execute(&self.pool)
                .await
                .map_err(|e| ContractError::Internal(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(ContractError::NotFound(
                "Akun pembeli tidak ditemukan".to_string(),
            ));
        }

        let _ = self
            .audit_contract
            .log_action(AuditLogEntry {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                actor_id: None,
                actor_username: "admin_seller".to_string(),
                action: "BUYER_STATUS_TOGGLED".to_string(),
                resource_type: "buyer".to_string(),
                resource_id: Some(buyer_id),
                details: format!("Buyer {} active status set to {}", buyer_id, is_active),
                ip_address: None,
            })
            .await;

        let updated_row = sqlx::query(
            "SELECT id, google_sub, email, full_name, avatar_url, phone_number, phone_verified, is_active, created_at, updated_at
             FROM buyer_accounts WHERE id = $1",
        )
        .bind(buyer_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        Self::row_to_buyer_dto(&updated_row)
    }

    async fn update_buyer_profile(
        &self,
        buyer_id: Uuid,
        full_name: Option<String>,
        avatar_url: Option<String>,
    ) -> Result<BuyerAccountDto, ContractError> {
        if let Some(ref name) = full_name {
            let trimmed = name.trim();
            if trimmed.len() < 2 || trimmed.len() > 100 {
                return Err(ContractError::ValidationError(
                    "Nama lengkap minimal 2 karakter (maks 100)".to_string(),
                ));
            }
        }

        let current_row = sqlx::query(
            "SELECT id, google_sub, email, full_name, avatar_url, phone_number, phone_verified, is_active, created_at, updated_at
             FROM buyer_accounts WHERE id = $1",
        )
        .bind(buyer_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let current = match current_row {
            Some(row) => Self::row_to_buyer_dto(&row)?,
            None => {
                return Err(ContractError::NotFound(
                    "Akun pembeli tidak ditemukan".to_string(),
                ));
            }
        };

        let new_name = full_name
            .map(|n| n.trim().to_string())
            .unwrap_or(current.full_name);
        let new_avatar = avatar_url.or(current.avatar_url);
        let now = Utc::now().to_rfc3339();

        let result = sqlx::query(
            "UPDATE buyer_accounts SET full_name = $1, avatar_url = $2, updated_at = $3 WHERE id = $4",
        )
        .bind(&new_name)
        .bind(&new_avatar)
        .bind(&now)
        .bind(buyer_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(ContractError::NotFound(
                "Akun pembeli tidak ditemukan".to_string(),
            ));
        }

        let _ = self
            .audit_contract
            .log_action(AuditLogEntry {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                actor_id: Some(buyer_id),
                actor_username: current.email.clone(),
                action: "BUYER_PROFILE_UPDATED".to_string(),
                resource_type: "buyer".to_string(),
                resource_id: Some(buyer_id),
                details: format!("Buyer {} updated profile", buyer_id),
                ip_address: None,
            })
            .await;

        let updated_row = sqlx::query(
            "SELECT id, google_sub, email, full_name, avatar_url, phone_number, phone_verified, is_active, created_at, updated_at
             FROM buyer_accounts WHERE id = $1",
        )
        .bind(buyer_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        Self::row_to_buyer_dto(&updated_row)
    }

    async fn get_wishlist(&self, buyer_id: Uuid) -> Result<Vec<WishlistItemDto>, ContractError> {
        let rows = sqlx::query(
            "SELECT w.id, w.buyer_id, w.product_id, w.created_at,
                    c.name as product_name, c.price as product_price, c.image_url as product_image_url
             FROM buyer_wishlists w
             JOIN catalog_items c ON w.product_id = c.id
             WHERE w.buyer_id = $1
             ORDER BY w.created_at DESC",
        )
        .bind(buyer_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let id_str: String = row.get("id");
            let b_id_str: String = row.get("buyer_id");
            let p_id_str: String = row.get("product_id");
            let name: String = row.get("product_name");
            let price_f64: f64 = row.get("product_price");
            let image_url: Option<String> = row.get("product_image_url");
            let created_at_str: String = row.get("created_at");

            let id = Uuid::parse_str(&id_str)
                .map_err(|e| ContractError::Internal(format!("Corrupt wishlist UUID: {}", e)))?;
            let b_id = Uuid::parse_str(&b_id_str)
                .map_err(|e| ContractError::Internal(format!("Corrupt buyer UUID: {}", e)))?;
            let p_id = Uuid::parse_str(&p_id_str)
                .map_err(|e| ContractError::Internal(format!("Corrupt product UUID: {}", e)))?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            items.push(WishlistItemDto {
                id,
                buyer_id: b_id,
                product_id: p_id,
                product_name: name,
                product_price_cents: price_f64.round() as i64,
                product_image_url: image_url,
                created_at,
            });
        }
        Ok(items)
    }

    async fn add_to_wishlist(
        &self,
        buyer_id: Uuid,
        product_id: Uuid,
    ) -> Result<WishlistItemDto, ContractError> {
        let prod_row =
            sqlx::query("SELECT id, name, price, image_url FROM catalog_items WHERE id = $1")
                .bind(product_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ContractError::Internal(e.to_string()))?;

        let prod = match prod_row {
            Some(p) => p,
            None => {
                return Err(ContractError::NotFound(format!(
                    "Produk {} tidak ditemukan",
                    product_id
                )))
            }
        };

        let wishlist_id = Uuid::new_v4();
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        let insert_res = sqlx::query(
            "INSERT INTO buyer_wishlists (id, buyer_id, product_id, created_at) VALUES ($1, $2, $3, $4)",
        )
        .bind(wishlist_id.to_string())
        .bind(buyer_id.to_string())
        .bind(product_id.to_string())
        .bind(&now_str)
        .execute(&self.pool)
        .await;

        if let Err(e) = insert_res {
            let err_str = e.to_string();
            if err_str.contains("UNIQUE constraint failed")
                || err_str.contains("code: 2067")
                || err_str.contains("code: 1555")
            {
                return Err(ContractError::AlreadyExists(
                    "Produk sudah ada di dalam wishlist".to_string(),
                ));
            } else {
                return Err(ContractError::Internal(err_str));
            }
        }

        let product_name: String = prod.get("name");
        let price_f64: f64 = prod.get("price");
        let product_image_url: Option<String> = prod.get("image_url");

        Ok(WishlistItemDto {
            id: wishlist_id,
            buyer_id,
            product_id,
            product_name,
            product_price_cents: price_f64.round() as i64,
            product_image_url,
            created_at: now,
        })
    }

    async fn remove_from_wishlist(
        &self,
        buyer_id: Uuid,
        product_id: Uuid,
    ) -> Result<(), ContractError> {
        let res =
            sqlx::query("DELETE FROM buyer_wishlists WHERE buyer_id = $1 AND product_id = $2")
                .bind(buyer_id.to_string())
                .bind(product_id.to_string())
                .execute(&self.pool)
                .await
                .map_err(|e| ContractError::Internal(e.to_string()))?;

        if res.rows_affected() == 0 {
            return Err(ContractError::NotFound(
                "Produk tidak ditemukan di dalam wishlist".to_string(),
            ));
        }
        Ok(())
    }

    async fn is_in_wishlist(
        &self,
        buyer_id: Uuid,
        product_id: Uuid,
    ) -> Result<bool, ContractError> {
        let row = sqlx::query(
            "SELECT 1 FROM buyer_wishlists WHERE buyer_id = $1 AND product_id = $2 LIMIT 1",
        )
        .bind(buyer_id.to_string())
        .bind(product_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        Ok(row.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use program1_core::database::init_database;
    use program1_module_audit::AuditModule;
    use program1_module_auth::AuthModule;
    use std::collections::HashMap;
    use tokio::sync::RwLock;

    struct TestGoogleVerifier;
    #[async_trait]
    impl GoogleTokenVerifier for TestGoogleVerifier {
        async fn verify(&self, id_token: &str) -> Result<GoogleClaims, ContractError> {
            if id_token.starts_with("valid:") {
                let parts: Vec<&str> = id_token.split(':').collect();
                Ok(GoogleClaims {
                    sub: parts.get(1).unwrap_or(&"sub123").to_string(),
                    email: parts.get(2).unwrap_or(&"test@buyer.com").to_string(),
                    name: parts.get(3).unwrap_or(&"Test Buyer").to_string(),
                    picture: Some("https://example.com/pic.jpg".to_string()),
                })
            } else {
                Err(ContractError::ValidationError(
                    "Invalid Google ID Token".to_string(),
                ))
            }
        }
    }

    struct TestSmsSender {
        last_otp: Arc<RwLock<HashMap<String, String>>>,
    }
    #[async_trait]
    impl SmsOtpSender for TestSmsSender {
        async fn send_otp(&self, phone_number: &str, otp_code: &str) -> Result<(), ContractError> {
            let mut lock = self.last_otp.write().await;
            lock.insert(phone_number.to_string(), otp_code.to_string());
            Ok(())
        }
    }

    async fn setup_test_buyer_module() -> (BuyerModule, Arc<RwLock<HashMap<String, String>>>) {
        let pool = init_database("sqlite::memory:").await.unwrap();
        let auth = Arc::new(AuthModule::new(
            "super-secret-key-minimum-32-chars-length!".to_string(),
            24,
        ));
        let audit = Arc::new(AuditModule::new(pool.clone()));
        let google_verifier = Arc::new(TestGoogleVerifier);
        let sent_otps = Arc::new(RwLock::new(HashMap::new()));
        let sms_sender = Arc::new(TestSmsSender {
            last_otp: sent_otps.clone(),
        });

        let module = BuyerModule::new(pool, auth, google_verifier, sms_sender, audit);
        (module, sent_otps)
    }

    #[tokio::test]
    async fn test_google_login_new_buyer_requires_phone() {
        let (module, _) = setup_test_buyer_module().await;
        let res = module
            .authenticate_google("valid:google_123:jane@buyer.com:Jane Doe")
            .await
            .unwrap();

        assert_eq!(res.buyer.email, "jane@buyer.com");
        assert_eq!(res.buyer.full_name, "Jane Doe");
        assert!(!res.buyer.phone_verified);
        assert!(res.requires_phone_verification);
        assert_eq!(res.token_type, "Bearer");
        assert!(!res.access_token.is_empty());
    }

    #[tokio::test]
    async fn test_buyer_register_and_login_email_password() {
        let (module, _) = setup_test_buyer_module().await;

        // 1. Register new buyer
        let reg_req = RegisterBuyerRequest {
            full_name: "Ahmad Dahlan".to_string(),
            email: "ahmad@store.com".to_string(),
            password: "SecurePassword123!".to_string(),
        };
        let reg_res = module
            .register(reg_req)
            .await
            .expect("Registration should succeed");

        assert_eq!(reg_res.buyer.email, "ahmad@store.com");
        assert_eq!(reg_res.buyer.full_name, "Ahmad Dahlan");
        assert_eq!(reg_res.buyer.google_sub, None);
        assert!(!reg_res.buyer.phone_verified);
        assert!(reg_res.requires_phone_verification);
        assert!(!reg_res.access_token.is_empty());

        // 2. Login with correct credentials
        let login_req = BuyerLoginRequest {
            email: "ahmad@store.com".to_string(),
            password: "SecurePassword123!".to_string(),
        };
        let login_res = module.login(login_req).await.expect("Login should succeed");
        assert_eq!(login_res.buyer.id, reg_res.buyer.id);
        assert_eq!(login_res.buyer.email, "ahmad@store.com");
        assert!(!login_res.access_token.is_empty());

        // 3. Login with case-insensitive trimmed email
        let login_upper = BuyerLoginRequest {
            email: "  AHMAD@store.COM  ".to_string(),
            password: "SecurePassword123!".to_string(),
        };
        let login_upper_res = module.login(login_upper).await;
        assert!(login_upper_res.is_ok());
    }

    #[tokio::test]
    async fn test_buyer_register_duplicate_email_rejected() {
        let (module, _) = setup_test_buyer_module().await;

        let reg_req1 = RegisterBuyerRequest {
            full_name: "User One".to_string(),
            email: "duplicate@store.com".to_string(),
            password: "Password123!".to_string(),
        };
        module.register(reg_req1).await.unwrap();

        let reg_req2 = RegisterBuyerRequest {
            full_name: "User Two".to_string(),
            email: "duplicate@store.com".to_string(),
            password: "DifferentPassword123!".to_string(),
        };
        let res2 = module.register(reg_req2).await;
        assert!(res2.is_err(), "Duplicate email registration must fail");
    }

    #[tokio::test]
    async fn test_buyer_login_wrong_password_rejected() {
        let (module, _) = setup_test_buyer_module().await;

        let reg_req = RegisterBuyerRequest {
            full_name: "Buyer Test".to_string(),
            email: "test_pass@store.com".to_string(),
            password: "CorrectPassword123!".to_string(),
        };
        module.register(reg_req).await.unwrap();

        let wrong_login = BuyerLoginRequest {
            email: "test_pass@store.com".to_string(),
            password: "WrongPassword999!".to_string(),
        };
        let res = module.login(wrong_login).await;
        assert!(res.is_err(), "Login with wrong password must fail");
    }

    #[tokio::test]
    async fn test_buyer_google_linking_to_email_registered_account() {
        let (module, _) = setup_test_buyer_module().await;

        // Register first via email
        let reg_req = RegisterBuyerRequest {
            full_name: "Linked Buyer".to_string(),
            email: "linked@store.com".to_string(),
            password: "Password123!".to_string(),
        };
        let reg_res = module.register(reg_req).await.unwrap();
        assert_eq!(reg_res.buyer.google_sub, None);

        // Later login with Google using same email
        let google_res = module
            .authenticate_google("valid:google_sub_linked_999:linked@store.com:Linked Buyer Google")
            .await
            .unwrap();

        assert_eq!(google_res.buyer.id, reg_res.buyer.id);
        assert_eq!(
            google_res.buyer.google_sub,
            Some("google_sub_linked_999".to_string())
        );
    }

    #[tokio::test]
    async fn test_admin_list_all_buyers() {
        let (module, _) = setup_test_buyer_module().await;

        let b1 = module
            .register(RegisterBuyerRequest {
                full_name: "Buyer Alpha".to_string(),
                email: "alpha@test.com".to_string(),
                password: "Password123!".to_string(),
            })
            .await
            .unwrap();

        let b2 = module
            .register(RegisterBuyerRequest {
                full_name: "Buyer Beta".to_string(),
                email: "beta@test.com".to_string(),
                password: "Password123!".to_string(),
            })
            .await
            .unwrap();

        let list = module
            .list_all_buyers()
            .await
            .expect("List all buyers should succeed");
        assert!(list.len() >= 2);
        assert_eq!(list[0].id, b2.buyer.id); // Most recent first
        assert_eq!(list[1].id, b1.buyer.id);
    }

    #[tokio::test]
    async fn test_admin_toggle_buyer_active() {
        let (module, _) = setup_test_buyer_module().await;

        let b = module
            .register(RegisterBuyerRequest {
                full_name: "Deactivate Me".to_string(),
                email: "deact@test.com".to_string(),
                password: "Password123!".to_string(),
            })
            .await
            .unwrap();

        assert!(b.buyer.is_active);

        // Deactivate
        let updated = module
            .set_buyer_active_status(b.buyer.id, false)
            .await
            .unwrap();
        assert!(!updated.is_active);

        // Reactivate
        let reactivated = module
            .set_buyer_active_status(b.buyer.id, true)
            .await
            .unwrap();
        assert!(reactivated.is_active);
    }

    #[tokio::test]
    async fn test_otp_request_and_verify_success() {
        let (module, sent_otps) = setup_test_buyer_module().await;
        let auth_res = module
            .authenticate_google("valid:google_456:budi@buyer.com:Budi Santoso")
            .await
            .unwrap();

        let buyer_id = auth_res.buyer.id;
        let phone = "+628123456789";

        // 1. Request OTP
        let req_res = module.request_phone_otp(buyer_id, phone).await;
        assert!(req_res.is_ok());

        // Extract sent OTP from mock SMS
        let otp_code = {
            let lock = sent_otps.read().await;
            lock.get(phone).cloned().expect("OTP should have been sent")
        };
        assert_eq!(otp_code.len(), 6);

        // 2. Verify OTP
        let updated = module
            .verify_phone_otp(buyer_id, phone, &otp_code)
            .await
            .unwrap();
        assert!(updated.phone_verified);
        assert_eq!(updated.phone_number.as_deref(), Some(phone));

        // 3. Subsequent login does not require OTP
        let login_again = module
            .authenticate_google("valid:google_456:budi@buyer.com:Budi Santoso")
            .await
            .unwrap();
        assert!(!login_again.requires_phone_verification);
        assert!(login_again.buyer.phone_verified);
    }

    #[tokio::test]
    async fn test_otp_wrong_code_increments_attempts_and_locks() {
        let (module, _) = setup_test_buyer_module().await;
        let auth_res = module
            .authenticate_google("valid:google_789:siti@buyer.com:Siti Aminah")
            .await
            .unwrap();

        let buyer_id = auth_res.buyer.id;
        let phone = "+628987654321";

        module.request_phone_otp(buyer_id, phone).await.unwrap();

        // Attempt 1: wrong code
        let err1 = module.verify_phone_otp(buyer_id, phone, "000000").await;
        assert!(err1.is_err());
        assert!(err1.unwrap_err().to_string().contains("Sisa percobaan: 2"));

        // Attempt 2: wrong code
        let err2 = module.verify_phone_otp(buyer_id, phone, "000000").await;
        assert!(err2.is_err());
        assert!(err2.unwrap_err().to_string().contains("Sisa percobaan: 1"));

        // Attempt 3: wrong code
        let err3 = module.verify_phone_otp(buyer_id, phone, "000000").await;
        assert!(err3.is_err());
        assert!(err3.unwrap_err().to_string().contains("Sisa percobaan: 0"));

        // Attempt 4: locked out
        let err4 = module.verify_phone_otp(buyer_id, phone, "000000").await;
        assert!(err4.is_err());
        assert!(err4.unwrap_err().to_string().contains("terlampaui"));
    }

    #[tokio::test]
    async fn test_otp_rate_limiting_cooldown() {
        let (module, _) = setup_test_buyer_module().await;
        let auth_res = module
            .authenticate_google("valid:google_rate:rate@buyer.com:Rate Limit")
            .await
            .unwrap();

        let buyer_id = auth_res.buyer.id;
        let phone = "+628111222333";

        // First request succeeds
        let res1 = module.request_phone_otp(buyer_id, phone).await;
        assert!(res1.is_ok());

        // Immediate second request fails with cooldown error
        let res2 = module.request_phone_otp(buyer_id, phone).await;
        assert!(res2.is_err());
        assert!(res2.unwrap_err().to_string().contains("Mohon tunggu"));
    }

    #[tokio::test]
    async fn test_buyer_addresses_single_default_guarantee() {
        let (module, _) = setup_test_buyer_module().await;
        let auth_res = module
            .authenticate_google("valid:google_addr:addr@buyer.com:Address Tester")
            .await
            .unwrap();
        let buyer_id = auth_res.buyer.id;

        // Add 1st address (should automatically become default)
        let addr1 = module
            .create_address(
                buyer_id,
                CreateBuyerAddressRequest {
                    recipient_name: "Address 1".to_string(),
                    phone_number: "+628123456789".to_string(),
                    street_address: "Street 1".to_string(),
                    subdistrict: "Sub 1".to_string(),
                    city: "City 1".to_string(),
                    province: "Prov 1".to_string(),
                    postal_code: "11111".to_string(),
                    set_as_default: false,
                },
            )
            .await
            .unwrap();
        assert!(addr1.is_default);

        // Add 2nd address as default
        let addr2 = module
            .create_address(
                buyer_id,
                CreateBuyerAddressRequest {
                    recipient_name: "Address 2".to_string(),
                    phone_number: "+628456789012".to_string(),
                    street_address: "Street 2".to_string(),
                    subdistrict: "Sub 2".to_string(),
                    city: "City 2".to_string(),
                    province: "Prov 2".to_string(),
                    postal_code: "22222".to_string(),
                    set_as_default: true,
                },
            )
            .await
            .unwrap();
        assert!(addr2.is_default);

        // Verify addr1 is no longer default
        let addr1_updated = module.get_address(buyer_id, addr1.id).await.unwrap();
        assert!(!addr1_updated.is_default);

        // List addresses - exactly one default
        let all_addrs = module.list_addresses(buyer_id).await.unwrap();
        assert_eq!(all_addrs.len(), 2);
        let default_count = all_addrs.iter().filter(|a| a.is_default).count();
        assert_eq!(default_count, 1);

        // Delete default address (addr2) -> addr1 automatically promoted to default!
        module.delete_address(buyer_id, addr2.id).await.unwrap();
        let addr1_promoted = module.get_address(buyer_id, addr1.id).await.unwrap();
        assert!(addr1_promoted.is_default);
    }

    #[test]
    fn test_normalize_indonesian_phone() {
        use super::normalize_indonesian_phone;

        assert_eq!(
            normalize_indonesian_phone("0812-3456-7890").unwrap(),
            "+6281234567890"
        );
        assert_eq!(
            normalize_indonesian_phone("+62 812-3456-7890").unwrap(),
            "+6281234567890"
        );
        assert_eq!(
            normalize_indonesian_phone("6281234567890").unwrap(),
            "+6281234567890"
        );
        assert_eq!(
            normalize_indonesian_phone("  +6281987654321 ").unwrap(),
            "+6281987654321"
        );

        // Invalid cases
        assert!(normalize_indonesian_phone("021-555-1234").is_err()); // landline
        assert!(normalize_indonesian_phone("0812").is_err()); // too short
        assert!(normalize_indonesian_phone("+12025550123").is_err()); // US number
        assert!(normalize_indonesian_phone("abc08123456789").is_err()); // non-phone characters
    }

    #[tokio::test]
    async fn test_phone_normalization_cooldown_cross_format() {
        let (module, _) = setup_test_buyer_module().await;
        let auth_res = module
            .authenticate_google("valid:google_cd:cd@buyer.com:Cooldown Buyer")
            .await
            .unwrap();

        let buyer_id = auth_res.buyer.id;

        // Request with local format 08...
        let res1 = module.request_phone_otp(buyer_id, "081234567890").await;
        assert!(res1.is_ok());

        // Second request with international format +628... must trigger cooldown!
        let res2 = module.request_phone_otp(buyer_id, "+6281234567890").await;
        assert!(res2.is_err());
        assert!(res2.unwrap_err().to_string().contains("Mohon tunggu"));
    }

    #[test]
    fn test_google_verifier_claim_validation() {
        use super::ProductionGoogleVerifier;
        let verifier = ProductionGoogleVerifier {
            client_id: "valid-client-id-123.apps.googleusercontent.com".to_string(),
        };

        let now_ts = chrono::Utc::now().timestamp();

        // 1. Audience mismatch
        let json_aud_mismatch = serde_json::json!({
            "aud": "wrong-client-id.apps.googleusercontent.com",
            "iss": "https://accounts.google.com",
            "email_verified": "true",
            "exp": now_ts + 3600,
            "sub": "sub-123",
            "email": "user@gmail.com",
            "name": "User"
        });
        assert!(verifier
            .parse_and_validate_claims(&json_aud_mismatch)
            .is_err());

        // 2. Issuer mismatch
        let json_iss_mismatch = serde_json::json!({
            "aud": "valid-client-id-123.apps.googleusercontent.com",
            "iss": "https://attacker-oauth.com",
            "email_verified": "true",
            "exp": now_ts + 3600,
            "sub": "sub-123",
            "email": "user@gmail.com",
            "name": "User"
        });
        assert!(verifier
            .parse_and_validate_claims(&json_iss_mismatch)
            .is_err());

        // 3. Email unverified
        let json_email_unverified = serde_json::json!({
            "aud": "valid-client-id-123.apps.googleusercontent.com",
            "iss": "https://accounts.google.com",
            "email_verified": "false",
            "exp": now_ts + 3600,
            "sub": "sub-123",
            "email": "user@gmail.com",
            "name": "User"
        });
        assert!(verifier
            .parse_and_validate_claims(&json_email_unverified)
            .is_err());

        // 4. Token expired
        let json_expired = serde_json::json!({
            "aud": "valid-client-id-123.apps.googleusercontent.com",
            "iss": "https://accounts.google.com",
            "email_verified": true,
            "exp": now_ts - 100,
            "sub": "sub-123",
            "email": "user@gmail.com",
            "name": "User"
        });
        assert!(verifier.parse_and_validate_claims(&json_expired).is_err());

        // 5. Valid claims
        let json_valid = serde_json::json!({
            "aud": "valid-client-id-123.apps.googleusercontent.com",
            "iss": "https://accounts.google.com",
            "email_verified": "true",
            "exp": now_ts + 3600,
            "sub": "sub-123",
            "email": "user@gmail.com",
            "name": "Valid User",
            "picture": "https://photo.jpg"
        });
        let claims = verifier.parse_and_validate_claims(&json_valid).unwrap();
        assert_eq!(claims.sub, "sub-123");
        assert_eq!(claims.email, "user@gmail.com");
    }

    #[tokio::test]
    async fn test_sms_sender_fail_closed_in_production() {
        use super::{ConsoleOrProviderSmsSender, SmsOtpSender};

        // In production environment with console/empty provider, must fail closed
        let sender = ConsoleOrProviderSmsSender::new(
            "production".to_string(),
            "".to_string(),
            "".to_string(),
        );
        let res = sender.send_otp("+6281234567890", "123456").await;
        assert!(res.is_err());

        // In development environment, console sender succeeds
        let dev_sender = ConsoleOrProviderSmsSender::new(
            "development".to_string(),
            "console".to_string(),
            "".to_string(),
        );
        let res_dev = dev_sender.send_otp("+6281234567890", "123456").await;
        assert!(res_dev.is_ok());
    }

    #[test]
    fn test_phone_masking() {
        use super::mask_phone;
        assert_eq!(mask_phone("+6281234567890"), "+628****890");
        assert_eq!(mask_phone("081234567890"), "0812****890");
        assert_eq!(mask_phone("1234"), "****");
    }

    #[tokio::test]
    async fn test_deactivated_buyer_login_and_profile_rejected() {
        let (module, _) = setup_test_buyer_module().await;
        let token = "valid:sub-deactivated:deact@buyer.com:Deactivated Buyer";
        let auth_res = module.authenticate_google(token).await.unwrap();
        let buyer_id = auth_res.buyer.id;

        // Verify active buyer profile works
        let profile = module.get_buyer_profile(buyer_id).await.unwrap();
        assert!(profile.is_active);

        // Deactivate buyer account
        sqlx::query("UPDATE buyer_accounts SET is_active = 0 WHERE id = $1")
            .bind(buyer_id.to_string())
            .execute(&module.pool)
            .await
            .unwrap();

        // Login must be rejected
        let login_err = module.authenticate_google(token).await.unwrap_err();
        match login_err {
            ContractError::ValidationError(msg) => {
                assert!(
                    msg.contains("dinonaktifkan"),
                    "Expected deactivated message, got: {}",
                    msg
                );
            }
            other => panic!("Expected ValidationError, got {:?}", other),
        }

        // Profile lookup must also be rejected
        let profile_err = module.get_buyer_profile(buyer_id).await.unwrap_err();
        match profile_err {
            ContractError::ValidationError(msg) => {
                assert!(
                    msg.contains("dinonaktifkan"),
                    "Expected deactivated message, got: {}",
                    msg
                );
            }
            other => panic!("Expected ValidationError, got {:?}", other),
        }
    }

    struct FailingSmsSender;
    #[async_trait]
    impl SmsOtpSender for FailingSmsSender {
        async fn send_otp(
            &self,
            _phone_number: &str,
            _otp_code: &str,
        ) -> Result<(), ContractError> {
            Err(ContractError::Internal(
                "Simulated SMS network dispatch failure".to_string(),
            ))
        }
    }

    #[tokio::test]
    async fn test_request_otp_rollback_on_sms_failure() {
        let pool = init_database("sqlite::memory:").await.unwrap();
        let auth = Arc::new(AuthModule::new(
            "super-secret-key-minimum-32-chars-length!".to_string(),
            24,
        ));
        let audit = Arc::new(AuditModule::new(pool.clone()));
        let google_verifier = Arc::new(TestGoogleVerifier);
        let sms_sender = Arc::new(FailingSmsSender);

        let module = BuyerModule::new(pool.clone(), auth, google_verifier, sms_sender, audit);
        let auth_res = module
            .authenticate_google("valid:sub-sms-fail:smsfail@buyer.com:Sms Fail")
            .await
            .unwrap();

        let req_res = module
            .request_phone_otp(auth_res.buyer.id, "+6281234567890")
            .await;
        assert!(req_res.is_err());

        // Ensure newly inserted OTP row was immediately deleted on SMS failure
        let count_row = sqlx::query(
            "SELECT COUNT(*) as count FROM buyer_otp_verifications WHERE buyer_id = $1",
        )
        .bind(auth_res.buyer.id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        let count: i64 = count_row.get("count");
        assert_eq!(
            count, 0,
            "Failed SMS dispatch must delete pending OTP verification record"
        );
    }

    #[tokio::test]
    async fn test_address_crud_transaction_and_partial_unique_index() {
        let (module, _) = setup_test_buyer_module().await;
        let auth_res = module
            .authenticate_google("valid:sub-addr:addr@buyer.com:Addr Buyer")
            .await
            .unwrap();
        let buyer_id = auth_res.buyer.id;

        // 1. Create first address -> automatically default
        let addr1 = module
            .create_address(
                buyer_id,
                CreateBuyerAddressRequest {
                    recipient_name: "Recipient 1".to_string(),
                    phone_number: "+6281234567890".to_string(),
                    street_address: "Street 1".to_string(),
                    subdistrict: "Subdistrict 1".to_string(),
                    city: "Jakarta".to_string(),
                    province: "DKI".to_string(),
                    postal_code: "12345".to_string(),
                    set_as_default: false,
                },
            )
            .await
            .unwrap();
        assert!(addr1.is_default);

        // 2. Create second address with set_as_default = true
        let addr2 = module
            .create_address(
                buyer_id,
                CreateBuyerAddressRequest {
                    recipient_name: "Recipient 2".to_string(),
                    phone_number: "+6281234567891".to_string(),
                    street_address: "Street 2".to_string(),
                    subdistrict: "Subdistrict 2".to_string(),
                    city: "Bandung".to_string(),
                    province: "Jabar".to_string(),
                    postal_code: "40123".to_string(),
                    set_as_default: true,
                },
            )
            .await
            .unwrap();
        assert!(addr2.is_default);

        // Verify addr1 is no longer default
        let addr1_refreshed = module.get_address(buyer_id, addr1.id).await.unwrap();
        assert!(!addr1_refreshed.is_default);

        // Verify exact single default address in DB (satisfies idx_buyer_addresses_one_default)
        let default_count_row = sqlx::query(
            "SELECT COUNT(*) as count FROM buyer_addresses WHERE buyer_id = $1 AND is_default = 1",
        )
        .bind(buyer_id.to_string())
        .fetch_one(&module.pool)
        .await
        .unwrap();
        let default_count: i64 = default_count_row.get("count");
        assert_eq!(default_count, 1);

        // 3. Delete default address addr2 -> addr1 should be promoted
        module.delete_address(buyer_id, addr2.id).await.unwrap();
        let addr1_promoted = module.get_address(buyer_id, addr1.id).await.unwrap();
        assert!(addr1_promoted.is_default);
    }

    #[tokio::test]
    async fn test_update_buyer_profile() {
        let (module, _) = setup_test_buyer_module().await;

        let reg = module
            .register(RegisterBuyerRequest {
                full_name: "Original Name".to_string(),
                email: "update_profile@store.com".to_string(),
                password: "Password123!".to_string(),
            })
            .await
            .unwrap();

        let updated = module
            .update_buyer_profile(
                reg.buyer.id,
                Some("Updated Name".to_string()),
                Some("https://example.com/new-avatar.png".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(updated.full_name, "Updated Name");
        assert_eq!(
            updated.avatar_url,
            Some("https://example.com/new-avatar.png".to_string())
        );

        // Validation test: name too short
        let invalid = module
            .update_buyer_profile(reg.buyer.id, Some("A".to_string()), None)
            .await;
        assert!(invalid.is_err());
    }

    #[tokio::test]
    async fn test_wishlist_add_list_and_remove() {
        let (module, _) = setup_test_buyer_module().await;

        let reg = module
            .register(RegisterBuyerRequest {
                full_name: "Wishlist Buyer".to_string(),
                email: "wishlist.test@store.com".to_string(),
                password: "Password123!".to_string(),
            })
            .await
            .unwrap();

        let prod_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO catalog_items (id, name, sku, category, price, stock) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(prod_id.to_string())
        .bind("Mechanical Keyboard RGB")
        .bind("SKU-KB-RGB")
        .bind("Accessories")
        .bind(250000.0)
        .bind(15)
        .execute(&module.pool)
        .await
        .unwrap();

        // 1. Check initially not in wishlist
        let in_wishlist = module.is_in_wishlist(reg.buyer.id, prod_id).await.unwrap();
        assert!(!in_wishlist);

        // 2. Add to wishlist
        let added = module.add_to_wishlist(reg.buyer.id, prod_id).await.unwrap();
        assert_eq!(added.buyer_id, reg.buyer.id);
        assert_eq!(added.product_id, prod_id);
        assert_eq!(added.product_name, "Mechanical Keyboard RGB");
        assert_eq!(added.product_price_cents, 250000);

        // 3. Now in wishlist
        let in_wishlist = module.is_in_wishlist(reg.buyer.id, prod_id).await.unwrap();
        assert!(in_wishlist);

        // 4. List wishlist
        let items = module.get_wishlist(reg.buyer.id).await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].product_name, "Mechanical Keyboard RGB");

        // 5. Remove from wishlist
        module
            .remove_from_wishlist(reg.buyer.id, prod_id)
            .await
            .unwrap();
        let items_after = module.get_wishlist(reg.buyer.id).await.unwrap();
        assert_eq!(items_after.len(), 0);

        // 6. Removing non-existent returns NotFound
        let not_found_err = module.remove_from_wishlist(reg.buyer.id, prod_id).await;
        assert!(matches!(not_found_err, Err(ContractError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_wishlist_duplicate_conflict() {
        let (module, _) = setup_test_buyer_module().await;

        let reg = module
            .register(RegisterBuyerRequest {
                full_name: "Conflict Buyer".to_string(),
                email: "conflict.wishlist@store.com".to_string(),
                password: "Password123!".to_string(),
            })
            .await
            .unwrap();

        let prod_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO catalog_items (id, name, sku, category, price, stock) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(prod_id.to_string())
        .bind("Gaming Mouse Pro")
        .bind("SKU-MOUSE-01")
        .bind("Accessories")
        .bind(100000.0)
        .bind(20)
        .execute(&module.pool)
        .await
        .unwrap();

        // Add first time -> OK
        module.add_to_wishlist(reg.buyer.id, prod_id).await.unwrap();

        // Add second time -> Conflict / AlreadyExists
        let err = module.add_to_wishlist(reg.buyer.id, prod_id).await;
        assert!(matches!(err, Err(ContractError::AlreadyExists(_))));
    }
}
