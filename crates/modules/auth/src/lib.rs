use async_trait::async_trait;
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use program1_contracts::{AuthContract, BuyerAccountDto, ContractError, JwtClaims, UserAccountDto};

#[derive(Clone)]
pub struct AuthModule {
    jwt_secret: String,
    token_expiry_hours: u64,
}

impl AuthModule {
    pub fn new(jwt_secret: String, token_expiry_hours: u64) -> Self {
        assert!(
            jwt_secret.len() >= 32,
            "JWT_SECRET must be at least 32 characters for security"
        );
        Self {
            jwt_secret,
            token_expiry_hours: if token_expiry_hours == 0 {
                24
            } else {
                token_expiry_hours
            },
        }
    }
}

#[async_trait]
impl AuthContract for AuthModule {
    fn generate_token(&self, user: &UserAccountDto) -> Result<String, ContractError> {
        let now = Utc::now().timestamp();
        let exp = now + (self.token_expiry_hours as i64 * 3600);

        let claims = JwtClaims {
            sub: user.id,
            username: user.username.clone(),
            role: user.role.clone(),
            accessible_menus: user.accessible_menus.clone(),
            exp,
            iat: now,
            user_type: "seller_staff".to_string(),
        };

        let header = Header::new(jsonwebtoken::Algorithm::HS256);

        encode(
            &header,
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| ContractError::Internal(format!("Failed to generate JWT: {}", e)))
    }

    fn generate_buyer_token(&self, buyer: &BuyerAccountDto) -> Result<String, ContractError> {
        let now = Utc::now().timestamp();
        let exp = now + (self.token_expiry_hours as i64 * 3600);

        let claims = JwtClaims {
            sub: buyer.id,
            username: buyer.email.clone(),
            role: "Buyer".to_string(),
            accessible_menus: vec![],
            exp,
            iat: now,
            user_type: "buyer".to_string(),
        };

        let header = Header::new(jsonwebtoken::Algorithm::HS256);

        encode(
            &header,
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| ContractError::Internal(format!("Failed to generate Buyer JWT: {}", e)))
    }

    fn validate_token(&self, token: &str) -> Result<JwtClaims, ContractError> {
        let mut validation = Validation::default();
        validation.validate_exp = true;

        decode::<JwtClaims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )
        .map(|token_data| token_data.claims)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                ContractError::ValidationError("Token has expired".to_string())
            }
            _ => ContractError::ValidationError(format!("Invalid token: {}", e)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn sample_user() -> UserAccountDto {
        UserAccountDto {
            id: Uuid::new_v4(),
            username: "admin_tester".to_string(),
            full_name: "Admin Tester".to_string(),
            role: "Super Admin".to_string(),
            accessible_menus: vec!["dashboard".to_string(), "orders".to_string()],
            is_active: true,
            created_at: Utc::now(),
        }
    }

    // 1. Test generate & validate token
    #[test]
    fn test_jwt_roundtrip() {
        let secret = "super-secret-key-minimum-32-chars-length!".to_string();
        let auth = AuthModule::new(secret, 24);
        let user = sample_user();

        let token = auth
            .generate_token(&user)
            .expect("Token generation should succeed");
        assert!(!token.is_empty());

        let claims = auth
            .validate_token(&token)
            .expect("Token validation should succeed");
        assert_eq!(claims.sub, user.id);
        assert_eq!(claims.username, user.username);
        assert_eq!(claims.role, user.role);
    }

    // 2. Test token expired
    #[test]
    fn test_expired_token_rejected() {
        let secret = "super-secret-key-minimum-32-chars-length!".to_string();
        let auth = AuthModule::new(secret.clone(), 24);
        let user = sample_user();

        // Create expired claims manually
        let now = Utc::now().timestamp();
        let expired_claims = JwtClaims {
            sub: user.id,
            username: user.username.clone(),
            role: user.role.clone(),
            accessible_menus: user.accessible_menus.clone(),
            exp: now - 3600, // 1 hour in the past
            iat: now - 7200,
            user_type: "seller_staff".to_string(),
        };

        let token = encode(
            &Header::default(),
            &expired_claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap();

        let result = auth.validate_token(&token);
        assert!(result.is_err());
        match result.unwrap_err() {
            ContractError::ValidationError(msg) => {
                assert!(msg.contains("expired") || msg.contains("ExpiredSignature"));
            }
            other => panic!(
                "Expected ValidationError for expired token, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_buyer_token_roundtrip() {
        let secret = "super-secret-key-minimum-32-chars-length!".to_string();
        let auth = AuthModule::new(secret, 24);
        let buyer_id = Uuid::new_v4();
        let buyer = BuyerAccountDto {
            id: buyer_id,
            google_sub: Some("google-sub-999".to_string()),
            email: "buyer_test@example.com".to_string(),
            full_name: "Buyer Test".to_string(),
            avatar_url: None,
            phone_number: Some("+628123456789".to_string()),
            phone_verified: true,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let token = auth
            .generate_buyer_token(&buyer)
            .expect("Buyer token should generate");
        let claims = auth
            .validate_token(&token)
            .expect("Buyer token should validate");
        assert_eq!(claims.sub, buyer_id);
        assert_eq!(claims.username, "buyer_test@example.com");
        assert!(claims.is_buyer());
        assert!(!claims.is_seller_staff());
    }

    // 3. Test invalid token format
    #[test]
    fn test_invalid_token_rejected() {
        let secret = "super-secret-key-minimum-32-chars-length!".to_string();
        let auth = AuthModule::new(secret, 24);

        let result = auth.validate_token("this.is.not.a.valid.jwt.token");
        assert!(result.is_err());
    }

    // 4. Test tampered token (wrong secret)
    #[test]
    fn test_tampered_token_rejected() {
        let secret_a = "secret-a-key-minimum-32-chars-length-1!".to_string();
        let secret_b = "secret-b-key-minimum-32-chars-length-2!".to_string();

        let auth_a = AuthModule::new(secret_a, 24);
        let auth_b = AuthModule::new(secret_b, 24);
        let user = sample_user();

        let token_from_a = auth_a.generate_token(&user).unwrap();
        let result_on_b = auth_b.validate_token(&token_from_a);
        assert!(result_on_b.is_err());
    }

    // 5. Test claims extraction
    #[test]
    fn test_claims_contain_user_info() {
        let secret = "super-secret-key-minimum-32-chars-length!".to_string();
        let auth = AuthModule::new(secret, 12);
        let user = sample_user();

        let token = auth.generate_token(&user).unwrap();
        let claims = auth.validate_token(&token).unwrap();

        assert_eq!(claims.sub, user.id);
        assert_eq!(claims.username, "admin_tester");
        assert_eq!(claims.role, "Super Admin");
        assert_eq!(claims.accessible_menus, vec!["dashboard", "orders"]);
        assert!(claims.exp > claims.iat);
    }
}
