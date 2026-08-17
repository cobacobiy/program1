use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;
use uuid::Uuid;

use program1_contracts::{
    ContractError, CreateProductRequest, ProductContract, ProductDto,
};

#[derive(Clone)]
pub struct ProductModule {
    store: Arc<RwLock<HashMap<Uuid, ProductDto>>>,
}

impl ProductModule {
    pub fn new() -> Self {
        let store = Arc::new(RwLock::new(HashMap::new()));

        let initial_products = vec![
            ProductDto {
                id: Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap(),
                name: "High-Performance Rust Server".to_string(),
                sku: "SKU-RUST-001".to_string(),
                price: 1250.00,
                stock: 25,
                created_at: Utc::now(),
            },
            ProductDto {
                id: Uuid::parse_str("10000000-0000-0000-0000-000000000002").unwrap(),
                name: "Modular Architecture Guide".to_string(),
                sku: "SKU-RUST-002".to_string(),
                price: 49.99,
                stock: 100,
                created_at: Utc::now(),
            },
            ProductDto {
                id: Uuid::parse_str("10000000-0000-0000-0000-000000000003").unwrap(),
                name: "WebAssembly UI Starter Kit".to_string(),
                sku: "SKU-RUST-003".to_string(),
                price: 89.50,
                stock: 50,
                created_at: Utc::now(),
            },
        ];

        let mut lock = store.blocking_write();
        for p in initial_products {
            lock.insert(p.id, p);
        }
        drop(lock);

        Self { store }
    }
}

impl Default for ProductModule {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProductContract for ProductModule {
    async fn get_product(&self, id: Uuid) -> Result<ProductDto, ContractError> {
        let lock = self.store.read().await;
        lock.get(&id)
            .cloned()
            .ok_or_else(|| ContractError::NotFound(format!("Product {}", id)))
    }

    async fn create_product(&self, req: CreateProductRequest) -> Result<ProductDto, ContractError> {
        if req.name.trim().is_empty() {
            return Err(ContractError::ValidationError("Product name cannot be empty".to_string()));
        }
        if req.price < 0.0 {
            return Err(ContractError::ValidationError("Price cannot be negative".to_string()));
        }

        let product = ProductDto {
            id: Uuid::new_v4(),
            name: req.name.trim().to_string(),
            sku: req.sku.trim().to_uppercase(),
            price: req.price,
            stock: req.stock,
            created_at: Utc::now(),
        };

        let mut lock = self.store.write().await;
        lock.insert(product.id, product.clone());
        tracing::info!(product_id = %product.id, sku = %product.sku, "Product created successfully");
        Ok(product)
    }

    async fn list_products(&self) -> Result<Vec<ProductDto>, ContractError> {
        let lock = self.store.read().await;
        let mut list: Vec<ProductDto> = lock.values().cloned().collect();
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(list)
    }

    async fn reserve_stock(&self, product_id: Uuid, quantity: u32) -> Result<(), ContractError> {
        let mut lock = self.store.write().await;
        let product = lock
            .get_mut(&product_id)
            .ok_or_else(|| ContractError::NotFound(format!("Product {}", product_id)))?;

        if product.stock < quantity {
            return Err(ContractError::InsufficientStock {
                product_id,
                requested: quantity,
                available: product.stock,
            });
        }

        product.stock -= quantity;
        tracing::info!(
            product_id = %product_id,
            remaining_stock = product.stock,
            reserved = quantity,
            "Stock reserved successfully"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_product_module_stock_reservation() {
        let module = ProductModule::new();
        let products = module.list_products().await.unwrap();
        let target = &products[0];

        let initial_stock = target.stock;
        module.reserve_stock(target.id, 5).await.unwrap();

        let updated = module.get_product(target.id).await.unwrap();
        assert_eq!(updated.stock, initial_stock - 5);

        // Test insufficient stock failure
        let res = module.reserve_stock(target.id, 99999).await;
        assert!(matches!(res, Err(ContractError::InsufficientStock { .. })));
    }
}
