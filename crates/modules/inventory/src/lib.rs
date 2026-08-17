use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;
use uuid::Uuid;

use program1_contracts::{
    CatalogContract, ContractError, InventoryContract, InventoryStockDto, SafetyStockLogDto,
};

#[derive(Clone)]
pub struct InventoryModule {
    stocks: Arc<RwLock<HashMap<Uuid, InventoryStockDto>>>,
    safety_logs: Arc<RwLock<HashMap<Uuid, Vec<SafetyStockLogDto>>>>,
    catalog_contract: Arc<dyn CatalogContract>,
}

impl InventoryModule {
    pub fn new(catalog_contract: Arc<dyn CatalogContract>) -> Self {
        Self {
            stocks: Arc::new(RwLock::new(HashMap::new())),
            safety_logs: Arc::new(RwLock::new(HashMap::new())),
            catalog_contract,
        }
    }

    fn calculate_available(warehouse: u32, locked: u32, spare: u32, promo: u32, safety: u32) -> u32 {
        warehouse.saturating_sub(locked + spare + promo + safety)
    }

    async fn ensure_product_initialized(&self, product_id: Uuid) -> Result<InventoryStockDto, ContractError> {
        let lock = self.stocks.read().await;
        if let Some(stock) = lock.get(&product_id) {
            return Ok(stock.clone());
        }
        drop(lock);

        let item = self.catalog_contract.get_item(product_id).await?;

        // Generate sample Ginee multi-type stock initial values
        let warehouse_stock = item.stock + 500;
        let spare_stock = 10;
        let locked_stock = 0;
        let promotion_stock = 0;
        let safety_stock = 15;
        let available_stock = Self::calculate_available(warehouse_stock, locked_stock, spare_stock, promotion_stock, safety_stock);

        let stock = InventoryStockDto {
            product_id: item.id,
            sku: item.sku,
            product_name: item.name,
            image_url: item.image_url,
            average_purchase_price: item.price * 0.65, // Sample cost price
            warehouse_stock,
            spare_stock,
            locked_stock,
            promotion_stock,
            safety_stock,
            available_stock,
            last_updated: Utc::now(),
        };

        let mut lock = self.stocks.write().await;
        lock.insert(product_id, stock.clone());
        Ok(stock)
    }
}

#[async_trait]
impl InventoryContract for InventoryModule {
    async fn get_all_stocks(&self) -> Result<Vec<InventoryStockDto>, ContractError> {
        let catalog_items = self.catalog_contract.list_items().await?;
        let mut result = Vec::new();
        for item in catalog_items {
            let stock = self.ensure_product_initialized(item.id).await?;
            result.push(stock);
        }
        Ok(result)
    }

    async fn get_stock(&self, product_id: Uuid) -> Result<InventoryStockDto, ContractError> {
        self.ensure_product_initialized(product_id).await
    }

    async fn reserve_stock(&self, product_id: Uuid, quantity: u32) -> Result<(), ContractError> {
        let _ = self.ensure_product_initialized(product_id).await?;

        let mut lock = self.stocks.write().await;
        let stock = lock.get_mut(&product_id).unwrap();

        if stock.available_stock < quantity {
            return Err(ContractError::InsufficientStock {
                product_id,
                requested: quantity,
                available: stock.available_stock,
            });
        }

        stock.locked_stock += quantity;
        stock.available_stock = Self::calculate_available(
            stock.warehouse_stock,
            stock.locked_stock,
            stock.spare_stock,
            stock.promotion_stock,
            stock.safety_stock,
        );
        stock.last_updated = Utc::now();

        tracing::info!(
            product_id = %product_id,
            locked = stock.locked_stock,
            available = stock.available_stock,
            "Stock locked for order checkout"
        );
        Ok(())
    }

    async fn update_safety_stock(
        &self,
        product_id: Uuid,
        new_safety_stock: u32,
        admin_note: String,
        updated_by: String,
    ) -> Result<InventoryStockDto, ContractError> {
        if admin_note.trim().is_empty() {
            return Err(ContractError::ValidationError(
                "Catatan Admin (Alasan Perubahan Safety Stock) wajib diisi!".to_string(),
            ));
        }

        let mut stock = self.ensure_product_initialized(product_id).await?;
        let old_safety_stock = stock.safety_stock;

        let log_entry = SafetyStockLogDto {
            id: Uuid::new_v4(),
            product_id,
            old_safety_stock,
            new_safety_stock,
            admin_note: admin_note.trim().to_string(),
            updated_by: if updated_by.trim().is_empty() { "Admin Ginee".to_string() } else { updated_by.trim().to_string() },
            timestamp: Utc::now(),
        };

        // Update stock
        let mut lock = self.stocks.write().await;
        stock.safety_stock = new_safety_stock;
        stock.available_stock = Self::calculate_available(
            stock.warehouse_stock,
            stock.locked_stock,
            stock.spare_stock,
            stock.promotion_stock,
            stock.safety_stock,
        );
        stock.last_updated = Utc::now();
        lock.insert(product_id, stock.clone());

        // Append log entry
        let mut logs_lock = self.safety_logs.write().await;
        logs_lock.entry(product_id).or_default().push(log_entry);

        tracing::info!(
            product_id = %product_id,
            old = old_safety_stock,
            new = new_safety_stock,
            note = %admin_note,
            "Safety stock updated with admin audit note"
        );

        Ok(stock)
    }

    async fn get_safety_stock_logs(&self, product_id: Uuid) -> Result<Vec<SafetyStockLogDto>, ContractError> {
        let logs_lock = self.safety_logs.read().await;
        let mut logs = logs_lock.get(&product_id).cloned().unwrap_or_default();
        logs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(logs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use program1_module_catalog::CatalogModule;

    #[tokio::test]
    async fn test_ginee_multi_stock_and_safety_logs() {
        let catalog = Arc::new(CatalogModule::new());
        let inventory = InventoryModule::new(catalog.clone());

        let items = catalog.list_items().await.unwrap();
        let target = &items[0];

        let stock = inventory.get_stock(target.id).await.unwrap();
        assert!(stock.warehouse_stock > 0);

        // Update safety stock with note
        let updated = inventory
            .update_safety_stock(target.id, 50, "Penyesuaian Promo 9.9".to_string(), "Admin Super".to_string())
            .await
            .unwrap();

        assert_eq!(updated.safety_stock, 50);

        let logs = inventory.get_safety_stock_logs(target.id).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].admin_note, "Penyesuaian Promo 9.9");
        assert_eq!(logs[0].old_safety_stock, 15);
        assert_eq!(logs[0].new_safety_stock, 50);
    }
}
