use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;
use uuid::Uuid;

use program1_contracts::{ContractError, CreateUserRequest, UserContract, UserDto};

#[derive(Clone)]
pub struct UserModule {
    store: Arc<RwLock<HashMap<Uuid, UserDto>>>,
}

impl UserModule {
    pub fn new() -> Self {
        let initial_users = vec![
            UserDto {
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                name: "Admin System".to_string(),
                email: "admin@program1.dev".to_string(),
                role: "Administrator".to_string(),
                created_at: Utc::now(),
            },
            UserDto {
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
                name: "Customer One".to_string(),
                email: "customer1@example.com".to_string(),
                role: "Customer".to_string(),
                created_at: Utc::now(),
            },
        ];

        let mut map = HashMap::new();
        for u in initial_users {
            map.insert(u.id, u);
        }

        Self {
            store: Arc::new(RwLock::new(map)),
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
    async fn get_user(&self, id: Uuid) -> Result<UserDto, ContractError> {
        let lock = self.store.read().await;
        lock.get(&id)
            .cloned()
            .ok_or_else(|| ContractError::NotFound(format!("User {}", id)))
    }

    async fn create_user(&self, req: CreateUserRequest) -> Result<UserDto, ContractError> {
        if req.name.trim().is_empty() {
            return Err(ContractError::ValidationError("User name cannot be empty".to_string()));
        }
        if !req.email.contains('@') {
            return Err(ContractError::ValidationError("Invalid email format".to_string()));
        }

        let user = UserDto {
            id: Uuid::new_v4(),
            name: req.name.trim().to_string(),
            email: req.email.trim().to_string(),
            role: req.role.unwrap_or_else(|| "Customer".to_string()),
            created_at: Utc::now(),
        };

        let mut lock = self.store.write().await;
        lock.insert(user.id, user.clone());
        tracing::info!(user_id = %user.id, name = %user.name, "User created successfully");
        Ok(user)
    }

    async fn list_users(&self) -> Result<Vec<UserDto>, ContractError> {
        let lock = self.store.read().await;
        let mut list: Vec<UserDto> = lock.values().cloned().collect();
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_module_crud() {
        let module = UserModule::new();
        let users = module.list_users().await.unwrap();
        assert!(users.len() >= 2);

        let new_user = module
            .create_user(CreateUserRequest {
                name: "Bob Dev".to_string(),
                email: "bob@rust.dev".to_string(),
                role: Some("Engineer".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(new_user.name, "Bob Dev");
        assert_eq!(new_user.email, "bob@rust.dev");

        let fetched = module.get_user(new_user.id).await.unwrap();
        assert_eq!(fetched.id, new_user.id);
    }
}
