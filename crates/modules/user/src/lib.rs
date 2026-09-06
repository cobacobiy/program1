use async_trait::async_trait;
use chrono::{DateTime, Utc};
use program1_contracts::{
    ActivateBreakGlassRequest, BreakGlassStatusDto, ContractError, CreateUserAccountRequest,
    RegisterUserRequest, UserAccountDto, UserContract,
};
use program1_core::database::DbPool;
use sqlx::Row;
use uuid::Uuid;

/// Password validation rules:
/// - Minimum 8 karakter
/// - Harus mengandung huruf besar, huruf kecil, dan angka
/// - TIDAK boleh sama dengan username
pub fn validate_password(password: &str, username: &str) -> Result<(), ContractError> {
    if password.len() < 8 {
        return Err(ContractError::ValidationError(
            "Password must be at least 8 characters long".to_string(),
        ));
    }

    let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());

    if !has_upper || !has_lower || !has_digit {
        return Err(ContractError::ValidationError(
            "Password must contain uppercase letters, lowercase letters, and digits".to_string(),
        ));
    }

    if password.trim().eq_ignore_ascii_case(username.trim()) {
        return Err(ContractError::ValidationError(
            "Password cannot be the same as username".to_string(),
        ));
    }

    Ok(())
}

#[derive(Clone)]
pub struct UserModule {
    pool: DbPool,
    dev_support_password: Option<String>,
}

impl UserModule {
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool,
            dev_support_password: None,
        }
    }

    pub fn with_dev_support_password(mut self, password: Option<String>) -> Self {
        self.dev_support_password = password;
        self
    }

    pub async fn seed_default_users(&self) -> Result<(), ContractError> {
        let all_menus = vec![
            "dashboard",
            "orders",
            "master_products",
            "channel_products",
            "purchases",
            "stocks",
            "warehouses",
            "promotions",
            "customers",
            "chat",
            "reports",
            "logistics",
            "finances",
            "integrations",
            "settings",
            "service",
        ];

        let admin_default_password =
            std::env::var("ADMIN_DEFAULT_PASSWORD").unwrap_or_else(|_| "admin123".to_string());
        let seed_password_hash = program1_core::auth::hash_password(&admin_default_password)
            .unwrap_or_else(|_| {
                "$argon2id$v=19$m=19456,t=2,p=1$placeholder$placeholder".to_string()
            });

        let dev_support_password = self
            .dev_support_password
            .clone()
            .or_else(|| std::env::var("DEV_SUPPORT_PASSWORD").ok())
            .unwrap_or_else(|| {
                format!(
                    "sec_rnd_{}_{}",
                    Uuid::new_v4().simple(),
                    Uuid::new_v4().simple()
                )
            });
        let dev_support_password_hash = program1_core::auth::hash_password(&dev_support_password)
            .unwrap_or_else(|_| {
                "$argon2id$v=19$m=19456,t=2,p=1$placeholder$placeholder".to_string()
            });

        let seed_accounts = vec![
            (
                "00000000-0000-0000-0000-000000000001",
                "admin",
                "Admin Super (Owner)",
                "Super Admin",
                serde_json::to_string(&all_menus).unwrap(),
                1,
            ),
            (
                "00000000-0000-0000-0000-000000000005",
                "admin_ops",
                "Budi Hartono (Admin Ops)",
                "Admin Operasional",
                serde_json::to_string(&all_menus).unwrap(),
                1,
            ),
            (
                "00000000-0000-0000-0000-000000000003",
                "manager_gudang",
                "Bambang W (Manager Gudang)",
                "Warehouse Manager",
                serde_json::to_string(&vec![
                    "dashboard",
                    "master_products",
                    "channel_products",
                    "stocks",
                    "warehouses",
                    "logistics",
                ])
                .unwrap(),
                1,
            ),
            (
                "00000000-0000-0000-0000-000000000006",
                "staff_gudang",
                "Joko Susilo (Staff Gudang)",
                "Staff Gudang & Stok",
                serde_json::to_string(&vec!["dashboard", "stocks", "master_products"]).unwrap(),
                1,
            ),
            (
                "00000000-0000-0000-0000-000000000004",
                "staff_finance",
                "Dewi Lestari (Staff Keuangan)",
                "Finance Officer",
                serde_json::to_string(&vec!["dashboard", "orders", "reports", "finances"]).unwrap(),
                1,
            ),
            (
                "00000000-0000-0000-0000-000000000002",
                "staff_cs",
                "Siti Rahma (Staff CS)",
                "Customer Support",
                serde_json::to_string(&vec!["dashboard", "orders", "customers", "chat", "service"])
                    .unwrap(),
                1,
            ),
            (
                "00000000-0000-0000-0000-000000000007",
                "dev_support",
                "Developer Support (Break-Glass)",
                "Developer Support",
                serde_json::to_string(&all_menus).unwrap(),
                0, // Break-glass: INACTIVE by default
            ),
        ];

        let now = Utc::now().to_rfc3339();
        for (id, username, full_name, role, menus, is_active) in seed_accounts {
            let pass_hash = if username == "dev_support" {
                &dev_support_password_hash
            } else {
                &seed_password_hash
            };
            sqlx::query(
                "INSERT OR IGNORE INTO user_accounts (id, username, password_hash, full_name, role, accessible_menus, is_active, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(id)
            .bind(username)
            .bind(pass_hash)
            .bind(full_name)
            .bind(role)
            .bind(menus)
            .bind(is_active)
            .bind(&now)
            .execute(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;
        }

        // Initialize break_glass_sessions record for dev_support
        sqlx::query(
            "INSERT OR IGNORE INTO break_glass_sessions (id, account_username, is_active, active_until, activated_by, reason, created_at)
             VALUES ('00000000-0000-0000-0000-000000000099', 'dev_support', 0, NULL, NULL, NULL, $1)",
        )
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        Ok(())
    }

    fn row_to_dto(row: &sqlx::sqlite::SqliteRow) -> Result<UserAccountDto, ContractError> {
        let id_str: String = row.get("id");
        let id = Uuid::parse_str(&id_str)
            .map_err(|e| ContractError::Internal(format!("Corrupt UUID in database: {}", e)))?;

        let username: String = row.get("username");
        let full_name: String = row.get("full_name");
        let role: String = row.get("role");
        let menus_json: String = row.get("accessible_menus");
        let mut accessible_menus: Vec<String> =
            serde_json::from_str(&menus_json).unwrap_or_default();
        if role.to_lowercase().contains("admin") {
            accessible_menus = vec![
                "dashboard".to_string(),
                "orders".to_string(),
                "master_products".to_string(),
                "channel_products".to_string(),
                "purchases".to_string(),
                "stocks".to_string(),
                "warehouses".to_string(),
                "promotions".to_string(),
                "customers".to_string(),
                "chat".to_string(),
                "reports".to_string(),
                "logistics".to_string(),
                "finances".to_string(),
                "integrations".to_string(),
                "settings".to_string(),
                "service".to_string(),
            ];
        }
        let is_active: i64 = row.get("is_active");
        let created_at_str: String = row.get("created_at");
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(UserAccountDto {
            id,
            username,
            full_name,
            role,
            accessible_menus,
            is_active: is_active != 0,
            created_at,
        })
    }
}

#[async_trait]
impl UserContract for UserModule {
    async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<UserAccountDto, ContractError> {
        let row = sqlx::query(
            "SELECT id, username, password_hash, full_name, role, accessible_menus, is_active, created_at
             FROM user_accounts WHERE LOWER(username) = LOWER($1)",
        )
        .bind(username.trim())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let row = match row {
            Some(r) => r,
            None => {
                return Err(ContractError::ValidationError(
                    "Invalid username or password".to_string(),
                ));
            }
        };

        let clean_user = username.trim().to_lowercase();
        if clean_user == "dev_support" {
            let bg_status = self.get_break_glass_status().await?;
            if !bg_status.is_active {
                return Err(ContractError::ValidationError(
                    "Akses Developer Support (Break-Glass) sedang nonaktif. Memerlukan otorisasi darurat dari Seller Owner.".to_string(),
                ));
            }
        }

        let password_hash: String = row.get("password_hash");
        let is_active: i64 = row.get("is_active");
        if is_active == 0 {
            return Err(ContractError::ValidationError(
                "Account is deactivated".to_string(),
            ));
        }

        let is_valid = program1_core::auth::verify_password(password, &password_hash)
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        if !is_valid {
            return Err(ContractError::ValidationError(
                "Invalid username or password".to_string(),
            ));
        }

        Self::row_to_dto(&row)
    }

    async fn get_account(&self, user_id: Uuid) -> Result<UserAccountDto, ContractError> {
        let row = sqlx::query(
            "SELECT id, username, full_name, role, accessible_menus, is_active, created_at
             FROM user_accounts WHERE id = $1",
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        match row {
            Some(r) => Self::row_to_dto(&r),
            None => Err(ContractError::NotFound(format!(
                "User account with ID {} not found",
                user_id
            ))),
        }
    }

    async fn list_accounts(&self) -> Result<Vec<UserAccountDto>, ContractError> {
        let rows = sqlx::query(
            "SELECT id, username, full_name, role, accessible_menus, is_active, created_at
             FROM user_accounts ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        rows.iter().map(Self::row_to_dto).collect()
    }

    async fn create_account(
        &self,
        req: CreateUserAccountRequest,
    ) -> Result<UserAccountDto, ContractError> {
        let clean_username = req.username.trim().to_lowercase();
        if clean_username.is_empty() {
            return Err(ContractError::ValidationError(
                "Username cannot be empty".to_string(),
            ));
        }

        // Check if username already exists
        let exists_row = sqlx::query(
            "SELECT COUNT(*) as count FROM user_accounts WHERE LOWER(username) = LOWER($1)",
        )
        .bind(&clean_username)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let exists_count: i64 = exists_row.get("count");
        if exists_count > 0 {
            return Err(ContractError::ValidationError(format!(
                "Username '{}' is already taken",
                req.username
            )));
        }

        let new_id = Uuid::new_v4();
        let now = Utc::now();
        let default_password =
            std::env::var("ADMIN_DEFAULT_PASSWORD").unwrap_or_else(|_| "admin123".to_string());
        let password_hash = program1_core::auth::hash_password(&default_password)
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let menus_json =
            serde_json::to_string(&req.accessible_menus).unwrap_or_else(|_| "[]".to_string());

        sqlx::query(
            "INSERT INTO user_accounts (id, username, password_hash, full_name, role, accessible_menus, is_active, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, 1, $7)",
        )
        .bind(new_id.to_string())
        .bind(&clean_username)
        .bind(&password_hash)
        .bind(req.full_name.trim())
        .bind(req.role.trim())
        .bind(menus_json)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let mut final_menus = req.accessible_menus;
        if req.role.to_lowercase().contains("admin") {
            final_menus = vec![
                "dashboard".to_string(),
                "orders".to_string(),
                "master_products".to_string(),
                "channel_products".to_string(),
                "purchases".to_string(),
                "stocks".to_string(),
                "warehouses".to_string(),
                "promotions".to_string(),
                "customers".to_string(),
                "chat".to_string(),
                "reports".to_string(),
                "logistics".to_string(),
                "finances".to_string(),
                "integrations".to_string(),
                "settings".to_string(),
                "service".to_string(),
            ];
        }

        Ok(UserAccountDto {
            id: new_id,
            username: clean_username,
            full_name: req.full_name,
            role: req.role,
            accessible_menus: final_menus,
            is_active: true,
            created_at: now,
        })
    }

    async fn update_permissions(
        &self,
        user_id: Uuid,
        accessible_menus: Vec<String>,
    ) -> Result<UserAccountDto, ContractError> {
        let menus_json =
            serde_json::to_string(&accessible_menus).unwrap_or_else(|_| "[]".to_string());

        let result = sqlx::query("UPDATE user_accounts SET accessible_menus = $1 WHERE id = $2")
            .bind(menus_json)
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(ContractError::NotFound(format!(
                "User account with ID {} not found",
                user_id
            )));
        }

        self.get_account(user_id).await
    }

    async fn register(&self, req: RegisterUserRequest) -> Result<UserAccountDto, ContractError> {
        let clean_username = req.username.trim().to_lowercase();
        if clean_username.is_empty() {
            return Err(ContractError::ValidationError(
                "Username cannot be empty".to_string(),
            ));
        }

        validate_password(&req.password, &clean_username)?;

        let exists_row = sqlx::query(
            "SELECT COUNT(*) as count FROM user_accounts WHERE LOWER(username) = LOWER($1)",
        )
        .bind(&clean_username)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let exists_count: i64 = exists_row.get("count");
        if exists_count > 0 {
            return Err(ContractError::ValidationError(format!(
                "Username '{}' is already registered",
                req.username
            )));
        }

        let password_hash = program1_core::auth::hash_password(&req.password)
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let new_id = Uuid::new_v4();
        let now = Utc::now();
        let role = if req.role.trim().is_empty() {
            "Staff".to_string()
        } else {
            req.role.trim().to_string()
        };
        let menus = if req.accessible_menus.is_empty() {
            vec!["dashboard".to_string(), "orders".to_string()]
        } else {
            req.accessible_menus
        };
        let menus_json = serde_json::to_string(&menus).unwrap_or_else(|_| "[]".to_string());

        sqlx::query(
            "INSERT INTO user_accounts (id, username, password_hash, full_name, role, accessible_menus, is_active, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, 1, $7)",
        )
        .bind(new_id.to_string())
        .bind(&clean_username)
        .bind(&password_hash)
        .bind(req.full_name.trim())
        .bind(&role)
        .bind(menus_json)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        Ok(UserAccountDto {
            id: new_id,
            username: clean_username,
            full_name: req.full_name,
            role,
            accessible_menus: menus,
            is_active: true,
            created_at: now,
        })
    }

    async fn activate_break_glass(
        &self,
        req: ActivateBreakGlassRequest,
        activated_by: &str,
    ) -> Result<BreakGlassStatusDto, ContractError> {
        let duration = req.duration_minutes.clamp(5, 240);
        let now = Utc::now();
        let until = now + chrono::Duration::minutes(duration as i64);
        let until_str = until.to_rfc3339();

        sqlx::query(
            "UPDATE break_glass_sessions
             SET is_active = 1, active_until = $1, activated_by = $2, reason = $3
             WHERE account_username = 'dev_support'",
        )
        .bind(&until_str)
        .bind(activated_by.trim())
        .bind(req.reason.trim())
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        sqlx::query("UPDATE user_accounts SET is_active = 1 WHERE LOWER(username) = 'dev_support'")
            .execute(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        Ok(BreakGlassStatusDto {
            is_active: true,
            active_until: Some(until),
            activated_by: Some(activated_by.trim().to_string()),
            reason: Some(req.reason.trim().to_string()),
            target_account: "dev_support".to_string(),
        })
    }

    async fn deactivate_break_glass(
        &self,
        _deactivated_by: &str,
        _reason: Option<String>,
    ) -> Result<BreakGlassStatusDto, ContractError> {
        sqlx::query(
            "UPDATE break_glass_sessions
             SET is_active = 0, active_until = NULL
             WHERE account_username = 'dev_support'",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        sqlx::query("UPDATE user_accounts SET is_active = 0 WHERE LOWER(username) = 'dev_support'")
            .execute(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        Ok(BreakGlassStatusDto {
            is_active: false,
            active_until: None,
            activated_by: None,
            reason: None,
            target_account: "dev_support".to_string(),
        })
    }

    async fn get_break_glass_status(&self) -> Result<BreakGlassStatusDto, ContractError> {
        let row = sqlx::query(
            "SELECT is_active, active_until, activated_by, reason FROM break_glass_sessions
             WHERE account_username = 'dev_support' ORDER BY created_at DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        match row {
            Some(r) => {
                let active: i64 = r.get("is_active");
                let until_str: Option<String> = r.get("active_until");
                let activated_by: Option<String> = r.get("activated_by");
                let reason: Option<String> = r.get("reason");

                let mut is_active = active == 1;
                let mut active_until = None;

                if let Some(ref u_str) = until_str {
                    if let Ok(u_dt) = DateTime::parse_from_rfc3339(u_str) {
                        let u_utc = u_dt.with_timezone(&Utc);
                        if Utc::now() > u_utc {
                            // Expired! Automatically mark inactive
                            let _ = self
                                .deactivate_break_glass(
                                    "system",
                                    Some("Session expired".to_string()),
                                )
                                .await;
                            is_active = false;
                            active_until = Some(u_utc);
                        } else {
                            active_until = Some(u_utc);
                        }
                    } else {
                        is_active = false;
                    }
                } else {
                    is_active = false;
                }

                Ok(BreakGlassStatusDto {
                    is_active,
                    active_until,
                    activated_by,
                    reason,
                    target_account: "dev_support".to_string(),
                })
            }
            None => Ok(BreakGlassStatusDto {
                is_active: false,
                active_until: None,
                activated_by: None,
                reason: None,
                target_account: "dev_support".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use program1_core::init_database;

    async fn create_test_user_module() -> UserModule {
        let pool = init_database("sqlite::memory:")
            .await
            .expect("In-memory SQLite init failed");
        let module = UserModule::new(pool)
            .with_dev_support_password(Some("test_dev_support_secret_token_12345!".to_string()));
        module.seed_default_users().await.expect("Seeding failed");
        module
    }

    #[tokio::test]
    async fn test_user_accounts_and_rbac() {
        let module = create_test_user_module().await;
        let accounts = module.list_accounts().await.unwrap();
        assert_eq!(accounts.len(), 7);

        let admin = &accounts[0];
        assert_eq!(admin.username, "admin");
        assert_eq!(admin.role, "Super Admin");
        assert_eq!(admin.accessible_menus.len(), 16);

        // Test create new account
        let new_acc = module
            .create_account(CreateUserAccountRequest {
                username: "operator_packing".to_string(),
                full_name: "Packing Staff".to_string(),
                role: "Warehouse Operator".to_string(),
                accessible_menus: vec!["logistics".to_string()],
            })
            .await
            .unwrap();

        assert_eq!(new_acc.username, "operator_packing");
        assert_eq!(new_acc.accessible_menus, vec!["logistics"]);

        // Test update permissions
        let updated = module
            .update_permissions(
                new_acc.id,
                vec!["logistics".to_string(), "orders".to_string()],
            )
            .await
            .unwrap();

        assert_eq!(
            updated.accessible_menus,
            vec!["logistics".to_string(), "orders".to_string()]
        );
    }

    #[tokio::test]
    async fn test_create_account_empty_username_validation() {
        let module = create_test_user_module().await;
        let result = module
            .create_account(CreateUserAccountRequest {
                username: "   ".to_string(),
                full_name: "Empty User".to_string(),
                role: "Staff".to_string(),
                accessible_menus: vec![],
            })
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ContractError::ValidationError(msg) => {
                assert_eq!(msg, "Username cannot be empty");
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[tokio::test]
    async fn test_password_validation_rules() {
        assert!(validate_password("short1A", "user").is_err());
        assert!(validate_password("alllowercase1", "user").is_err());
        assert!(validate_password("ALLUPPERCASE1", "user").is_err());
        assert!(validate_password("NoDigitsHere!", "user").is_err());
        assert!(validate_password("Admin123", "admin123").is_err());
        assert!(validate_password("ValidPass123", "other_user").is_ok());
    }

    #[tokio::test]
    async fn test_authenticate_valid_credentials() {
        let module = create_test_user_module().await;
        let result = module.authenticate("admin", "admin123").await;
        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.username, "admin");
        assert_eq!(user.role, "Super Admin");
    }

    #[tokio::test]
    async fn test_authenticate_wrong_password() {
        let module = create_test_user_module().await;
        let result = module.authenticate("admin", "wrongpassword").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ContractError::ValidationError(msg) => {
                assert_eq!(msg, "Invalid username or password");
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[tokio::test]
    async fn test_authenticate_nonexistent_user() {
        let module = create_test_user_module().await;
        let result = module.authenticate("nobody", "admin123").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ContractError::ValidationError(msg) => {
                assert_eq!(msg, "Invalid username or password");
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[tokio::test]
    async fn test_register_new_user() {
        let module = create_test_user_module().await;
        let req = RegisterUserRequest {
            username: "newuser".to_string(),
            password: "SecurePassword123".to_string(),
            full_name: "New User".to_string(),
            role: "Staff".to_string(),
            accessible_menus: vec!["dashboard".to_string(), "orders".to_string()],
        };

        let user = module
            .register(req)
            .await
            .expect("Registration should succeed");
        assert_eq!(user.username, "newuser");
        assert_eq!(user.full_name, "New User");
        assert_eq!(user.role, "Staff");

        let auth_res = module.authenticate("newuser", "SecurePassword123").await;
        assert!(auth_res.is_ok());
    }

    #[tokio::test]
    async fn test_register_duplicate_username() {
        let module = create_test_user_module().await;
        let req = RegisterUserRequest {
            username: "admin".to_string(),
            password: "SecurePassword123".to_string(),
            full_name: "Another Admin".to_string(),
            role: "Super Admin".to_string(),
            accessible_menus: vec![],
        };

        let result = module.register(req).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ContractError::ValidationError(msg) => {
                assert!(msg.contains("already"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[tokio::test]
    async fn test_developer_support_disabled_by_default_and_cannot_login() {
        let module = create_test_user_module().await;
        let status = module.get_break_glass_status().await.unwrap();
        assert!(!status.is_active);

        let dev_pass = "test_dev_support_secret_token_12345!";

        // Dev support cannot use admin123
        let wrong_pass_res = module.authenticate("dev_support", "admin123").await;
        assert!(wrong_pass_res.is_err());

        // Even with correct password, dev_support login fails when break-glass is inactive
        let login_res = module.authenticate("dev_support", dev_pass).await;
        assert!(login_res.is_err());
        assert!(login_res.unwrap_err().to_string().contains("Break-Glass"));
    }

    #[tokio::test]
    async fn test_break_glass_activation_and_deactivation() {
        let module = create_test_user_module().await;
        let dev_pass = "test_dev_support_secret_token_12345!";

        // 1. Activate break-glass
        let activated = module
            .activate_break_glass(
                ActivateBreakGlassRequest {
                    reason: "Emergency investigation of database connection pool exhaustion"
                        .to_string(),
                    duration_minutes: 60,
                },
                "admin",
            )
            .await
            .unwrap();

        assert!(activated.is_active);
        assert_eq!(activated.activated_by.as_deref(), Some("admin"));
        assert!(activated.active_until.is_some());

        // 2. dev_support cannot login with admin123
        let wrong_pass = module.authenticate("dev_support", "admin123").await;
        assert!(wrong_pass.is_err());

        // 3. dev_support can now authenticate with independent password
        let auth_res = module.authenticate("dev_support", dev_pass).await;
        assert!(auth_res.is_ok());
        let user = auth_res.unwrap();
        assert_eq!(user.username, "dev_support");
        assert_eq!(user.role, "Developer Support");

        // 4. Deactivate break-glass
        let deactivated = module
            .deactivate_break_glass("admin", Some("Incident resolved".to_string()))
            .await
            .unwrap();
        assert!(!deactivated.is_active);

        // 5. dev_support can no longer authenticate
        let re_auth = module.authenticate("dev_support", dev_pass).await;
        assert!(re_auth.is_err());
    }

    #[tokio::test]
    async fn test_dev_support_random_password_when_env_not_set() {
        let pool = init_database("sqlite::memory:").await.unwrap();
        let module = UserModule::new(pool);
        module.seed_default_users().await.unwrap();

        // Activate break-glass
        let _ = module
            .activate_break_glass(
                ActivateBreakGlassRequest {
                    reason: "Testing random password fallback".to_string(),
                    duration_minutes: 30,
                },
                "admin",
            )
            .await
            .unwrap();

        // Old hardcoded password must fail
        let res = module
            .authenticate("dev_support", "dev_support_secret_token_12345!")
            .await;
        assert!(
            res.is_err(),
            "Old hardcoded password must never succeed when env var is unset"
        );

        // Common defaults must fail
        let res2 = module.authenticate("dev_support", "admin123").await;
        assert!(res2.is_err());
    }

    #[tokio::test]
    async fn test_upgrade_existing_database_with_user_accounts_seeds_dev_support_and_breakglass() {
        let pool = init_database("sqlite::memory:").await.unwrap();

        // Simulate pre-existing database with user_accounts already populated by an older release
        sqlx::query(
            "INSERT INTO user_accounts (id, username, password_hash, full_name, role, accessible_menus, is_active, created_at)
             VALUES ('00000000-0000-0000-0000-000000000001', 'legacy_admin', 'hash', 'Legacy Admin', 'Super Admin', '[]', 1, '2025-01-01T00:00:00Z')"
        )
        .execute(&pool)
        .await
        .unwrap();

        let initial_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_accounts")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(initial_count, 1);

        // break_glass_sessions table is currently empty
        let initial_bg_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM break_glass_sessions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(initial_bg_count, 0);

        // Run seed_default_users on the existing database
        let module = UserModule::new(pool.clone())
            .with_dev_support_password(Some("test_upgrade_dev_pass_123!".to_string()));
        module
            .seed_default_users()
            .await
            .expect("Upgraded seeding should not fail");

        // Verify dev_support user account was successfully seeded
        let dev_user = sqlx::query(
            "SELECT id, username, role FROM user_accounts WHERE username = 'dev_support'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap();
        assert!(
            dev_user.is_some(),
            "dev_support must be added to user_accounts during upgrade"
        );

        // Verify break_glass_sessions row was successfully seeded
        let bg_row = sqlx::query("SELECT id, account_username, is_active FROM break_glass_sessions WHERE account_username = 'dev_support'")
            .fetch_optional(&pool)
            .await
            .unwrap();
        assert!(
            bg_row.is_some(),
            "break_glass_sessions row must be added during upgrade"
        );

        // Verify legacy_admin was untouched
        let legacy =
            sqlx::query("SELECT username FROM user_accounts WHERE username = 'legacy_admin'")
                .fetch_optional(&pool)
                .await
                .unwrap();
        assert!(
            legacy.is_some(),
            "legacy_admin must remain intact after upgrade"
        );
    }
}
