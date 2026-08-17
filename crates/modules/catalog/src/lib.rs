use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;
use uuid::Uuid;

use program1_contracts::{
    CatalogContract, CatalogItemDto, ContractError, CreateCatalogItemRequest,
};

#[derive(Clone)]
pub struct CatalogModule {
    store: Arc<RwLock<HashMap<Uuid, CatalogItemDto>>>,
}

impl CatalogModule {
    pub fn new() -> Self {
        let initial_items = vec![
            CatalogItemDto {
                id: Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap(),
                name: "AURA Wireless Mechanical Keyboard".to_string(),
                sku: "SKU-AURA-KB01".to_string(),
                category: "Peripherals".to_string(),
                price: 1450000.0,
                stock: 45,
                image_url: "https://images.unsplash.com/photo-1587829741301-dc798b83add3?auto=format&fit=crop&w=500&q=80".to_string(),
                description: "RGB Hotswap Gasket Mount Keyboard with Bluetooth 5.2 and 2.4GHz Receiver.".to_string(),
                created_at: Utc::now(),
            },
            CatalogItemDto {
                id: Uuid::parse_str("10000000-0000-0000-0000-000000000002").unwrap(),
                name: "AURA Ergonomic Precision Mouse".to_string(),
                sku: "SKU-AURA-MS02".to_string(),
                category: "Peripherals".to_string(),
                price: 780000.0,
                stock: 80,
                image_url: "https://images.unsplash.com/photo-1615663245857-ac93bb7c39e7?auto=format&fit=crop&w=500&q=80".to_string(),
                description: "26K DPI Optical Sensor with Dual Wireless & Type-C Charging Dock.".to_string(),
                created_at: Utc::now(),
            },
            CatalogItemDto {
                id: Uuid::parse_str("10000000-0000-0000-0000-000000000003").unwrap(),
                name: "AURA Ultra-Wide Glass Monitor Arm".to_string(),
                sku: "SKU-AURA-ARM03".to_string(),
                category: "Accessories".to_string(),
                price: 950000.0,
                stock: 30,
                image_url: "https://images.unsplash.com/photo-1527443224154-c4a3942d3acf?auto=format&fit=crop&w=500&q=80".to_string(),
                description: "Heavy Duty Gas Spring Arm supporting up to 49-inch Ultrawide Displays.".to_string(),
                created_at: Utc::now(),
            },
        ];

        let mut map = HashMap::new();
        for item in initial_items {
            map.insert(item.id, item);
        }

        Self {
            store: Arc::new(RwLock::new(map)),
        }
    }
}

impl Default for CatalogModule {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CatalogContract for CatalogModule {
    async fn list_items(&self) -> Result<Vec<CatalogItemDto>, ContractError> {
        let lock = self.store.read().await;
        let mut list: Vec<CatalogItemDto> = lock.values().cloned().collect();
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(list)
    }

    async fn get_item(&self, id: Uuid) -> Result<CatalogItemDto, ContractError> {
        let lock = self.store.read().await;
        lock.get(&id)
            .cloned()
            .ok_or_else(|| ContractError::NotFound(format!("Catalog Item {}", id)))
    }

    async fn create_item(&self, req: CreateCatalogItemRequest) -> Result<CatalogItemDto, ContractError> {
        if req.name.trim().is_empty() {
            return Err(ContractError::ValidationError("Item name cannot be empty".to_string()));
        }
        if req.price < 0.0 {
            return Err(ContractError::ValidationError("Price cannot be negative".to_string()));
        }

        let item = CatalogItemDto {
            id: Uuid::new_v4(),
            name: req.name.trim().to_string(),
            sku: req.sku.trim().to_uppercase(),
            category: req.category.trim().to_string(),
            price: req.price,
            stock: req.stock,
            image_url: req.image_url.unwrap_or_else(|| "https://via.placeholder.com/500".to_string()),
            description: req.description.unwrap_or_else(|| "Product description".to_string()),
            created_at: Utc::now(),
        };

        let mut lock = self.store.write().await;
        lock.insert(item.id, item.clone());
        tracing::info!(id = %item.id, sku = %item.sku, "Catalog item created");
        Ok(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_catalog_crud() {
        let module = CatalogModule::new();
        let items = module.list_items().await.unwrap();
        assert!(items.len() >= 3);

        let created = module
            .create_item(CreateCatalogItemRequest {
                name: "Desk Mat XL".to_string(),
                sku: "SKU-MAT-99".to_string(),
                category: "Accessories".to_string(),
                price: 250000.0,
                stock: 20,
                image_url: None,
                description: None,
            })
            .await
            .unwrap();

        assert_eq!(created.name, "Desk Mat XL");
        let fetched = module.get_item(created.id).await.unwrap();
        assert_eq!(fetched.sku, "SKU-MAT-99");
    }
}
