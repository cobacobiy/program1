use async_trait::async_trait;
use chrono::{DateTime, Utc};
use program1_contracts::{
    ContractError, CreateUserAccountRequest, RegisterUserRequest, UserAccountDto, UserContract,
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
}

impl UserModule {
    pub fn new(pool: DbPool) -> Self {
        let module = Self { pool };
        // Trigger non-blocking seed in background / sync check
        let module_clone = module.clone();
        tokio::spawn(async move {
            let _ = module_clone.seed_default_users().await;
        });
        module
    }

    pub async fn seed_default_users(&self) -> Result<(), ContractError> {
        let count_row = sqlx::query("SELECT COUNT(*) as count FROM user_accounts")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let count: i64 = count_row.get("count");
        if count > 0 {
            return Ok(());
        }

        let all_menus = vec![
            "dashboard", "orders", "master_products", "channel_products", "purchases",
            "stocks", "warehouses", "promotions", "customers", "chat", "reports",
            "logistics", "finances", "integrations", "settings", "service",
        ];

        let admin_default_password = std::env::var("ADMIN_DEFAULT_PASSWORD")
            .unwrap_or_else(|_| "admin123".to_string());
        let seed_password_hash = program1_core::auth::hash_password(&admin_default_password)
            .unwrap_or_else(|_| "$argon2id$v=19$m=19456,t=2,p=1$placeholder$placeholder".to_string());

        let seed_accounts = vec![
            (
                "00000000-0000-0000-0000-000000000001",
                "admin",
                "Admin Super (Owner)",
                "Super Admin",
                serde_json::to_string(&all_menus).unwrap(),
            ),
            (
                "00000000-0000-0000-0000-000000000002",
                "staff_cs",
                "Siti Rahma (Staff CS)",
                "Customer Support",
                serde_json::to_string(&vec!["dashboard", "orders", "customers", "chat", "service"]).unwrap(),
            ),
            (
                "00000000-0000-0000-0000-000000000003",
                "staff_gudang",
                "Bambang W (Staff Gudang)",
                "Warehouse Manager",
                serde_json::to_string(&vec!["dashboard", "master_products", "stocks", "warehouses", "logistics"]).unwrap(),
            ),
            (
                "00000000-0000-0000-0000-000000000004",
                "staff_finance",
                "Dewi Lestari (Staff Keuangan)",
                "Finance Officer",
                serde_json::to_string(&vec!["dashboard", "orders", "reports", "finances"]).unwrap(),
            ),
        ];

        for (id, username, full_name, role, menus) in seed_accounts {
            let _ = sqlx::query(
                "INSERT OR IGNORE INTO user_accounts (id, username, password_hash, full_name, role, accessible_menus, is_active, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, 1, $7)",
            )
            .bind(id)
            .bind(username)
            .bind(&seed_password_hash)
            .bind(full_name)
            .bind(role)
            .bind(menus)
            .bind(Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await;
        }

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
        let accessible_menus: Vec<String> = serde_json::from_str(&menus_json).unwrap_or_default();
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
        let exists_row = sqlx::query("SELECT COUNT(*) as count FROM user_accounts WHERE LOWER(username) = LOWER($1)")
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
        let default_password = std::env::var("ADMIN_DEFAULT_PASSWORD")
            .unwrap_or_else(|_| "admin123".to_string());
        let password_hash = program1_core::auth::hash_password(&default_password)
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let menus_json = serde_json::to_string(&req.accessible_menus).unwrap_or_else(|_| "[]".to_string());

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

        Ok(UserAccountDto {
            id: new_id,
            username: clean_username,
            full_name: req.full_name,
            role: req.role,
            accessible_menus: req.accessible_menus,
            is_active: true,
            created_at: now,
        })
    }

    async fn update_permissions(
        &self,
        user_id: Uuid,
        accessible_menus: Vec<String>,
    ) -> Result<UserAccountDto, ContractError> {
        let menus_json = serde_json::to_string(&accessible_menus).unwrap_or_else(|_| "[]".to_string());

        let result = sqlx::query(
            "UPDATE user_accounts SET accessible_menus = $1 WHERE id = $2",
        )
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

        let exists_row = sqlx::query("SELECT COUNT(*) as count FROM user_accounts WHERE LOWER(username) = LOWER($1)")
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
        let role = if req.role.trim().is_empty() { "Staff".to_string() } else { req.role.trim().to_string() };
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use program1_core::init_database;

    async fn create_test_user_module() -> UserModule {
        let pool = init_database("sqlite::memory:").await.expect("In-memory SQLite init failed");
        let module = UserModule::new(pool);
        module.seed_default_users().await.expect("Seeding failed");
        module
    }

    #[tokio::test]
    async fn test_user_accounts_and_rbac() {
        let module = create_test_user_module().await;
        let accounts = module.list_accounts().await.unwrap();
        assert_eq!(accounts.len(), 4);

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

        let user = module.register(req).await.expect("Registration should succeed");
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

}
