use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;
use uuid::Uuid;

use program1_contracts::{
    ContractError, CreateUserAccountRequest, RegisterUserRequest, UserAccountDto, UserContract,
};

/// Internal only — TIDAK di-export
#[derive(Clone, Debug)]
struct UserAccountInternal {
    pub dto: UserAccountDto,
    pub password_hash: String, // Argon2id hash
}

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
    accounts: Arc<RwLock<HashMap<Uuid, UserAccountInternal>>>,
    username_index: Arc<RwLock<HashMap<String, Uuid>>>,
}

impl UserModule {
    pub fn new() -> Self {
        let all_menus = vec![
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

        let admin_default_password = std::env::var("ADMIN_DEFAULT_PASSWORD")
            .unwrap_or_else(|_| "admin123".to_string());
        let seed_password_hash = program1_core::auth::hash_password(&admin_default_password)
            .unwrap_or_else(|_| "$argon2id$v=19$m=19456,t=2,p=1$placeholder$placeholder".to_string());

        let seed_accounts = vec![
            UserAccountDto {
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                username: "admin".to_string(),
                full_name: "Admin Super (Owner)".to_string(),
                role: "Super Admin".to_string(),
                accessible_menus: all_menus.clone(),
                is_active: true,
                created_at: Utc::now(),
            },
            UserAccountDto {
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
                username: "staff_cs".to_string(),
                full_name: "Siti Rahma (Staff CS)".to_string(),
                role: "Customer Support".to_string(),
                accessible_menus: vec![
                    "dashboard".to_string(),
                    "orders".to_string(),
                    "customers".to_string(),
                    "chat".to_string(),
                    "service".to_string(),
                ],
                is_active: true,
                created_at: Utc::now(),
            },
            UserAccountDto {
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
                username: "staff_gudang".to_string(),
                full_name: "Bambang W (Staff Gudang)".to_string(),
                role: "Warehouse Manager".to_string(),
                accessible_menus: vec![
                    "dashboard".to_string(),
                    "master_products".to_string(),
                    "stocks".to_string(),
                    "warehouses".to_string(),
                    "logistics".to_string(),
                ],
                is_active: true,
                created_at: Utc::now(),
            },
            UserAccountDto {
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000004").unwrap(),
                username: "staff_finance".to_string(),
                full_name: "Dewi Lestari (Staff Keuangan)".to_string(),
                role: "Finance Officer".to_string(),
                accessible_menus: vec![
                    "dashboard".to_string(),
                    "orders".to_string(),
                    "reports".to_string(),
                    "finances".to_string(),
                ],
                is_active: true,
                created_at: Utc::now(),
            },
        ];

        let mut accounts_map = HashMap::new();
        let mut index_map = HashMap::new();

        for acc in seed_accounts {
            index_map.insert(acc.username.to_lowercase(), acc.id);
            accounts_map.insert(
                acc.id,
                UserAccountInternal {
                    dto: acc,
                    password_hash: seed_password_hash.clone(),
                },
            );
        }

        Self {
            accounts: Arc::new(RwLock::new(accounts_map)),
            username_index: Arc::new(RwLock::new(index_map)),
        }
    }
}

impl Default for UserModule {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UserContract for UserModule {
    async fn list_accounts(&self) -> Result<Vec<UserAccountDto>, ContractError> {
        let lock = self.accounts.read().await;
        let mut list: Vec<UserAccountDto> = lock.values().map(|u| u.dto.clone()).collect();
        list.sort_by(|a, b| a.full_name.cmp(&b.full_name));
        Ok(list)
    }

    async fn get_account(&self, id: Uuid) -> Result<UserAccountDto, ContractError> {
        let lock = self.accounts.read().await;
        lock.get(&id)
            .map(|u| u.dto.clone())
            .ok_or_else(|| ContractError::NotFound(format!("User Account {}", id)))
    }

    async fn create_account(&self, req: CreateUserAccountRequest) -> Result<UserAccountDto, ContractError> {
        let clean_username = req.username.trim().to_lowercase();
        if clean_username.is_empty() {
            return Err(ContractError::ValidationError("Username cannot be empty".to_string()));
        }

        let index_lock = self.username_index.read().await;
        if index_lock.contains_key(&clean_username) {
            return Err(ContractError::ValidationError(format!(
                "Username '{}' is already taken",
                clean_username
            )));
        }
        drop(index_lock);

        let default_password = std::env::var("ADMIN_DEFAULT_PASSWORD")
            .unwrap_or_else(|_| "admin123".to_string());
        let password_hash = program1_core::auth::hash_password(&default_password)
            .map_err(ContractError::Internal)?;

        let acc = UserAccountDto {
            id: Uuid::new_v4(),
            username: clean_username.clone(),
            full_name: req.full_name.trim().to_string(),
            role: req.role.trim().to_string(),
            accessible_menus: req.accessible_menus,
            is_active: true,
            created_at: Utc::now(),
        };

        let internal = UserAccountInternal {
            dto: acc.clone(),
            password_hash,
        };

        let mut acc_lock = self.accounts.write().await;
        let mut idx_lock = self.username_index.write().await;

        acc_lock.insert(acc.id, internal);
        idx_lock.insert(clean_username, acc.id);

        tracing::info!(id = %acc.id, username = %acc.username, "User Account Created");
        Ok(acc)
    }

    async fn update_permissions(&self, id: Uuid, accessible_menus: Vec<String>) -> Result<UserAccountDto, ContractError> {
        let mut lock = self.accounts.write().await;
        let acc_internal = lock
            .get_mut(&id)
            .ok_or_else(|| ContractError::NotFound(format!("User Account {}", id)))?;

        acc_internal.dto.accessible_menus = accessible_menus;
        tracing::info!(id = %id, permissions_count = acc_internal.dto.accessible_menus.len(), "Permissions updated");
        Ok(acc_internal.dto.clone())
    }

    async fn authenticate(&self, username: &str, password: &str) -> Result<UserAccountDto, ContractError> {
        let clean_username = username.trim().to_lowercase();
        let idx_lock = self.username_index.read().await;
        let user_id = idx_lock
            .get(&clean_username)
            .copied()
            .ok_or_else(|| ContractError::NotFound(format!("User '{}' not found", username)))?;
        drop(idx_lock);

        let acc_lock = self.accounts.read().await;
        let internal = acc_lock
            .get(&user_id)
            .ok_or_else(|| ContractError::NotFound(format!("User '{}' not found", username)))?;

        let is_valid = program1_core::auth::verify_password(password, &internal.password_hash)
            .map_err(ContractError::Internal)?;

        if !is_valid {
            return Err(ContractError::ValidationError("Invalid password".to_string()));
        }

        if !internal.dto.is_active {
            return Err(ContractError::ValidationError("User account is inactive".to_string()));
        }

        tracing::info!(id = %internal.dto.id, username = %internal.dto.username, "User authenticated successfully");
        Ok(internal.dto.clone())
    }

    async fn register(&self, req: RegisterUserRequest) -> Result<UserAccountDto, ContractError> {
        let clean_username = req.username.trim().to_lowercase();
        if clean_username.is_empty() {
            return Err(ContractError::ValidationError("Username cannot be empty".to_string()));
        }

        validate_password(&req.password, &clean_username)?;

        let idx_lock = self.username_index.read().await;
        if idx_lock.contains_key(&clean_username) {
            return Err(ContractError::ValidationError(format!(
                "Username '{}' is already taken",
                clean_username
            )));
        }
        drop(idx_lock);

        let password_hash = program1_core::auth::hash_password(&req.password)
            .map_err(ContractError::Internal)?;

        let acc = UserAccountDto {
            id: Uuid::new_v4(),
            username: clean_username.clone(),
            full_name: req.full_name.trim().to_string(),
            role: if req.role.trim().is_empty() {
                "User".to_string()
            } else {
                req.role.trim().to_string()
            },
            accessible_menus: req.accessible_menus,
            is_active: true,
            created_at: Utc::now(),
        };

        let internal = UserAccountInternal {
            dto: acc.clone(),
            password_hash,
        };

        let mut acc_lock = self.accounts.write().await;
        let mut idx_lock = self.username_index.write().await;

        acc_lock.insert(acc.id, internal);
        idx_lock.insert(clean_username, acc.id);

        tracing::info!(id = %acc.id, username = %acc.username, "User registered successfully");
        Ok(acc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 1. Test password hashing & verification
    #[test]
    fn test_hash_and_verify_password() {
        let raw_pass = "SecurePass123!";
        let hash = program1_core::auth::hash_password(raw_pass).expect("Should hash successfully");
        assert_ne!(raw_pass, hash);
        assert!(program1_core::auth::verify_password(raw_pass, &hash).unwrap());
        assert!(!program1_core::auth::verify_password("WrongPassword123", &hash).unwrap());
    }

    // 2. Test login dengan credential benar
    #[tokio::test]
    async fn test_authenticate_valid_credentials() {
        let module = UserModule::new();
        let user = module.authenticate("admin", "admin123").await;
        assert!(user.is_ok());
        let user = user.unwrap();
        assert_eq!(user.username, "admin");
        assert_eq!(user.role, "Super Admin");
    }

    // 3. Test login dengan password salah
    #[tokio::test]
    async fn test_authenticate_wrong_password() {
        let module = UserModule::new();
        let result = module.authenticate("admin", "wrong_password").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ContractError::ValidationError(msg) => assert_eq!(msg, "Invalid password"),
            other => panic!("Expected ValidationError, got {:?}", other),
        }
    }

    // 4. Test login dengan username tidak ada
    #[tokio::test]
    async fn test_authenticate_nonexistent_user() {
        let module = UserModule::new();
        let result = module.authenticate("nonexistent_user", "admin123").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ContractError::NotFound(_) => {}
            other => panic!("Expected NotFound error, got {:?}", other),
        }
    }

    // 5. Test register user baru
    #[tokio::test]
    async fn test_register_new_user() {
        let module = UserModule::new();
        let req = RegisterUserRequest {
            username: "new_manager".to_string(),
            password: "SecurePassword123".to_string(),
            full_name: "New Store Manager".to_string(),
            role: "Manager".to_string(),
            accessible_menus: vec!["dashboard".to_string(), "orders".to_string()],
        };

        let created = module.register(req).await;
        assert!(created.is_ok());
        let user = created.unwrap();
        assert_eq!(user.username, "new_manager");

        // Authenticate with new credentials
        let auth_res = module.authenticate("new_manager", "SecurePassword123").await;
        assert!(auth_res.is_ok());
    }

    // 6. Test register dengan username duplikat
    #[tokio::test]
    async fn test_register_duplicate_username() {
        let module = UserModule::new();
        let req = RegisterUserRequest {
            username: "admin".to_string(), // already exists
            password: "AdminNewPass123".to_string(),
            full_name: "Duplicate Admin".to_string(),
            role: "Admin".to_string(),
            accessible_menus: vec![],
        };

        let result = module.register(req).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ContractError::ValidationError(msg) => {
                assert!(msg.contains("already taken"));
            }
            other => panic!("Expected ValidationError, got {:?}", other),
        }
    }

    // 7. Test password validation rules
    #[test]
    fn test_password_validation_rules() {
        // Too short (< 8 chars)
        let res_short = validate_password("Short1!", "user1");
        assert!(res_short.is_err());

        // Missing uppercase
        let res_no_upper = validate_password("lowercase123!", "user1");
        assert!(res_no_upper.is_err());

        // Missing lowercase
        let res_no_lower = validate_password("UPPERCASE123!", "user1");
        assert!(res_no_lower.is_err());

        // Missing digit
        let res_no_digit = validate_password("NoDigitsHere!", "user1");
        assert!(res_no_digit.is_err());

        // Same as username
        let res_same = validate_password("MyUserPassword1", "MyUserPassword1");
        assert!(res_same.is_err());

        // Valid password
        let res_valid = validate_password("ValidPassword123", "user1");
        assert!(res_valid.is_ok());
    }

    #[tokio::test]
    async fn test_user_accounts_and_rbac() {
        let module = UserModule::new();
        let accounts = module.list_accounts().await.unwrap();
        assert_eq!(accounts.len(), 4);

        // Verify admin seed has all 16 menus
        let admin = accounts.iter().find(|a| a.username == "admin").unwrap();
        assert_eq!(admin.accessible_menus.len(), 16);

        // Update permissions for CS staff
        let cs_staff = accounts.iter().find(|a| a.username == "staff_cs").unwrap();
        let updated = module
            .update_permissions(cs_staff.id, vec!["dashboard".to_string(), "chat".to_string(), "service".to_string()])
            .await
            .unwrap();

        assert_eq!(updated.accessible_menus, vec!["dashboard", "chat", "service"]);

        // Create new user account
        let new_user = module
            .create_account(CreateUserAccountRequest {
                username: "staff_promotions".to_string(),
                full_name: "Rian Marketing".to_string(),
                role: "Marketing Manager".to_string(),
                accessible_menus: vec!["dashboard".to_string(), "promotions".to_string()],
            })
            .await
            .unwrap();

        assert_eq!(new_user.username, "staff_promotions");
        assert_eq!(new_user.accessible_menus, vec!["dashboard", "promotions"]);

        let all_after_create = module.list_accounts().await.unwrap();
        assert_eq!(all_after_create.len(), 5);
    }

    #[tokio::test]
    async fn test_create_account_empty_username_validation() {
        let module = UserModule::new();
        let res = module
            .create_account(CreateUserAccountRequest {
                username: "   ".to_string(),
                full_name: "Test Invalid".to_string(),
                role: "Tester".to_string(),
                accessible_menus: vec![],
            })
            .await;

        assert!(res.is_err());
    }
}
