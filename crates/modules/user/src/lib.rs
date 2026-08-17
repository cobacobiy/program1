use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;
use uuid::Uuid;

use program1_contracts::{
    ContractError, CreateUserAccountRequest, UserAccountDto, UserContract,
};

#[derive(Clone)]
pub struct UserModule {
    accounts: Arc<RwLock<HashMap<Uuid, UserAccountDto>>>,
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

        let mut map = HashMap::new();
        for acc in seed_accounts {
            map.insert(acc.id, acc);
        }

        Self {
            accounts: Arc::new(RwLock::new(map)),
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
        let mut list: Vec<UserAccountDto> = lock.values().cloned().collect();
        list.sort_by(|a, b| a.full_name.cmp(&b.full_name));
        Ok(list)
    }

    async fn get_account(&self, id: Uuid) -> Result<UserAccountDto, ContractError> {
        let lock = self.accounts.read().await;
        lock.get(&id)
            .cloned()
            .ok_or_else(|| ContractError::NotFound(format!("User Account {}", id)))
    }

    async fn create_account(&self, req: CreateUserAccountRequest) -> Result<UserAccountDto, ContractError> {
        if req.username.trim().is_empty() {
            return Err(ContractError::ValidationError("Username cannot be empty".to_string()));
        }

        let acc = UserAccountDto {
            id: Uuid::new_v4(),
            username: req.username.trim().to_lowercase(),
            full_name: req.full_name.trim().to_string(),
            role: req.role.trim().to_string(),
            accessible_menus: req.accessible_menus,
            is_active: true,
            created_at: Utc::now(),
        };

        let mut lock = self.accounts.write().await;
        lock.insert(acc.id, acc.clone());
        tracing::info!(id = %acc.id, username = %acc.username, "User Account Created");
        Ok(acc)
    }

    async fn update_permissions(&self, id: Uuid, accessible_menus: Vec<String>) -> Result<UserAccountDto, ContractError> {
        let mut lock = self.accounts.write().await;
        let acc = lock
            .get_mut(&id)
            .ok_or_else(|| ContractError::NotFound(format!("User Account {}", id)))?;

        acc.accessible_menus = accessible_menus;
        tracing::info!(id = %id, permissions_count = acc.accessible_menus.len(), "Permissions updated");
        Ok(acc.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
