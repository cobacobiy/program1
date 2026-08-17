use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;
use uuid::Uuid;

use program1_contracts::{
    CatalogContract, ContractError, InventoryContract, InventoryStockDto,
};

#[derive(Clone)]
pub struct InventoryModule {
    store: Arc<RwLock<HashMap<Uuid, InventoryStockDto>>>,
    catalog_contract: Arc<dyn CatalogContract>,
}

impl InventoryModule {
    pub fn new(catalog_contract: Arc<dyn CatalogContract>) -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            catalog_contract,
        }
    }

    async fn ensure_product_initialized(&self, product_id: Uuid) -> Result<InventoryStockDto, ContractError> {
        let lock = self.store.read().await;
        if let Some(stock) = lock.get(&product_id) {
            return Ok(stock.clone());
        }
        drop(lock);

        // Fetch product from catalog contract
        let item = self.catalog_contract.get_item(product_id).await?;
        let stock = InventoryStockDto {
            product_id: item.id,
            sku: item.sku,
            available_stock: item.stock,
            reserved_stock: 0,
            total_stock: item.stock,
            last_updated: Utc::now(),
        };

        let mut lock = self.store.write().await;
        lock.insert(product_id, stock.clone());
        Ok(stock)
    }
}

#[async_trait]
impl InventoryContract for InventoryModule {
    async fn get_stock(&self, product_id: Uuid) -> Result<InventoryStockDto, ContractError> {
        self.ensure_product_initialized(product_id).await
    }

    async fn reserve_stock(&self, product_id: Uuid, quantity: u32) -> Result<(), ContractError> {
        let _ = self.ensure_product_initialized(product_id).await?;

        let mut lock = self.store.write().await;
        let stock = lock.get_mut(&product_id).unwrap();

        if stock.available_stock < quantity {
            return Err(ContractError::InsufficientStock {
                product_id,
                requested: quantity,
                available: stock.available_stock,
            });
        }

        stock.available_stock -= quantity;
        stock.reserved_stock += quantity;
        stock.last_updated = Utc::now();

        tracing::info!(
            product_id = %product_id,
            available = stock.available_stock,
            reserved = stock.reserved_stock,
            "Stock reserved successfully"
        );
        Ok(())
    }

    async fn update_stock(&self, product_id: Uuid, new_total_stock: u32) -> Result<InventoryStockDto, ContractError> {
        let mut stock = self.ensure_product_initialized(product_id).await?;

        let mut lock = self.store.write().await;
        stock.total_stock = new_total_stock;
        stock.available_stock = new_total_stock.saturating_sub(stock.reserved_stock);
        stock.last_updated = Utc::now();

        lock.insert(product_id, stock.clone());
        tracing::info!(product_id = %product_id, new_total = new_total_stock, "Stock updated");
        Ok(stock)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use program1_module_catalog::CatalogModule;

    #[tokio::test]
    async fn test_inventory_stock_reservation() {
        let catalog = Arc::new(CatalogModule::new());
        let inventory = InventoryModule::new(catalog.clone());

        let items = catalog.list_items().await.unwrap();
        let target = &items[0];

        let stock = inventory.get_stock(target.id).await.unwrap();
        assert_eq!(stock.available_stock, target.stock);

        inventory.reserve_stock(target.id, 5).await.unwrap();

        let updated = inventory.get_stock(target.id).await.unwrap();
        assert_eq!(updated.available_stock, target.stock - 5);
        assert_eq!(updated.reserved_stock, 5);
    }
}
