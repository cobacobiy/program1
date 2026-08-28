use std::sync::Arc;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use program1_contracts::{
    CatalogContract, ContractError, InventoryContract, InventoryStockDto, SafetyStockLogDto,
};
use program1_core::database::DbPool;
use sqlx::Row;
use uuid::Uuid;

#[derive(Clone)]
pub struct InventoryModule {
    pool: DbPool,
    catalog_contract: Arc<dyn CatalogContract>,
}

impl InventoryModule {
    pub fn new(pool: DbPool, catalog_contract: Arc<dyn CatalogContract>) -> Self {
        Self {
            pool,
            catalog_contract,
        }
    }

    fn calculate_available(warehouse: u32, locked: u32, spare: u32, promo: u32, safety: u32) -> u32 {
        warehouse.saturating_sub(locked + spare + promo + safety)
    }

    fn row_to_stock_dto(row: &sqlx::sqlite::SqliteRow) -> Result<InventoryStockDto, ContractError> {
        let product_id_str: String = row.get("product_id");
        let product_id = Uuid::parse_str(&product_id_str)
            .map_err(|e| ContractError::Internal(format!("Invalid UUID in inventory: {}", e)))?;

        let sku: String = row.get("sku");
        let product_name: String = row.get("product_name");
        let image_url: String = row.get("image_url");
        let avg_price: f64 = row.get("average_purchase_price");
        let warehouse: i64 = row.get("warehouse_stock");
        let spare: i64 = row.get("spare_stock");
        let locked: i64 = row.get("locked_stock");
        let promo: i64 = row.get("promotion_stock");
        let safety: i64 = row.get("safety_stock");
        let available: i64 = row.get("available_stock");
        let last_updated_str: String = row.get("last_updated");
        let last_updated = DateTime::parse_from_rfc3339(&last_updated_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(InventoryStockDto {
            product_id,
            sku,
            product_name,
            image_url,
            average_purchase_price: avg_price,
            warehouse_stock: warehouse as u32,
            spare_stock: spare as u32,
            locked_stock: locked as u32,
            promotion_stock: promo as u32,
            safety_stock: safety as u32,
            available_stock: available as u32,
            last_updated,
        })
    }

    async fn ensure_product_initialized(&self, product_id: Uuid) -> Result<InventoryStockDto, ContractError> {
        let existing = sqlx::query(
            "SELECT product_id, sku, product_name, image_url, average_purchase_price,
                    warehouse_stock, spare_stock, locked_stock, promotion_stock, safety_stock,
                    available_stock, last_updated
             FROM inventory_stocks WHERE product_id = $1",
        )
        .bind(product_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        if let Some(row) = existing {
            return Self::row_to_stock_dto(&row);
        }

        let item = self.catalog_contract.get_item(product_id).await?;

        // Generate sample Ginee multi-type stock initial values
        let warehouse_stock = item.stock + 500;
        let spare_stock = 10;
        let locked_stock = 0;
        let promotion_stock = 0;
        let safety_stock = 15;
        let available_stock = Self::calculate_available(warehouse_stock, locked_stock, spare_stock, promotion_stock, safety_stock);
        let now = Utc::now();
        let cost_price = item.price * 0.65;

        sqlx::query(
            "INSERT OR IGNORE INTO inventory_stocks (
                product_id, sku, product_name, image_url, average_purchase_price,
                warehouse_stock, spare_stock, locked_stock, promotion_stock, safety_stock,
                available_stock, last_updated
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(product_id.to_string())
        .bind(&item.sku)
        .bind(&item.name)
        .bind(&item.image_url)
        .bind(cost_price)
        .bind(warehouse_stock as i64)
        .bind(spare_stock as i64)
        .bind(locked_stock as i64)
        .bind(promotion_stock as i64)
        .bind(safety_stock as i64)
        .bind(available_stock as i64)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        Ok(InventoryStockDto {
            product_id: item.id,
            sku: item.sku,
            product_name: item.name,
            image_url: item.image_url,
            average_purchase_price: cost_price,
            warehouse_stock,
            spare_stock,
            locked_stock,
            promotion_stock,
            safety_stock,
            available_stock,
            last_updated: now,
        })
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
        let current_stock = self.ensure_product_initialized(product_id).await?;

        if current_stock.available_stock < quantity {
            return Err(ContractError::InsufficientStock {
                product_id,
                requested: quantity,
                available: current_stock.available_stock,
            });
        }

        let new_locked = current_stock.locked_stock + quantity;
        let new_available = Self::calculate_available(
            current_stock.warehouse_stock,
            new_locked,
            current_stock.spare_stock,
            current_stock.promotion_stock,
            current_stock.safety_stock,
        );
        let now = Utc::now();

        let result = sqlx::query(
            "UPDATE inventory_stocks
             SET locked_stock = $1, available_stock = $2, last_updated = $3
             WHERE product_id = $4 AND available_stock >= $5",
        )
        .bind(new_locked as i64)
        .bind(new_available as i64)
        .bind(now.to_rfc3339())
        .bind(product_id.to_string())
        .bind(quantity as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(ContractError::InsufficientStock {
                product_id,
                requested: quantity,
                available: current_stock.available_stock,
            });
        }

        tracing::info!(
            product_id = %product_id,
            locked = new_locked,
            available = new_available,
            "Stock locked in database for order checkout"
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

        let stock = self.ensure_product_initialized(product_id).await?;
        let old_safety_stock = stock.safety_stock;
        let now = Utc::now();
        let log_id = Uuid::new_v4();
        let operator = if updated_by.trim().is_empty() {
            "Admin Ginee".to_string()
        } else {
            updated_by.trim().to_string()
        };

        let new_available = Self::calculate_available(
            stock.warehouse_stock,
            stock.locked_stock,
            stock.spare_stock,
            stock.promotion_stock,
            new_safety_stock,
        );

        // Update database stock
        sqlx::query(
            "UPDATE inventory_stocks
             SET safety_stock = $1, available_stock = $2, last_updated = $3
             WHERE product_id = $4",
        )
        .bind(new_safety_stock as i64)
        .bind(new_available as i64)
        .bind(now.to_rfc3339())
        .bind(product_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        // Insert audit log
        sqlx::query(
            "INSERT INTO safety_stock_logs (id, product_id, old_safety_stock, new_safety_stock, admin_note, updated_by, timestamp)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(log_id.to_string())
        .bind(product_id.to_string())
        .bind(old_safety_stock as i64)
        .bind(new_safety_stock as i64)
        .bind(admin_note.trim())
        .bind(&operator)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        tracing::info!(
            product_id = %product_id,
            old = old_safety_stock,
            new = new_safety_stock,
            note = %admin_note,
            "Safety stock updated in database with audit note"
        );

        self.get_stock(product_id).await
    }

    async fn get_safety_stock_logs(&self, product_id: Uuid) -> Result<Vec<SafetyStockLogDto>, ContractError> {
        let rows = sqlx::query(
            "SELECT id, product_id, old_safety_stock, new_safety_stock, admin_note, updated_by, timestamp
             FROM safety_stock_logs WHERE product_id = $1 ORDER BY timestamp DESC",
        )
        .bind(product_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let mut logs = Vec::new();
        for r in rows {
            let id_str: String = r.get("id");
            let id = Uuid::parse_str(&id_str)
                .map_err(|e| ContractError::Internal(format!("Invalid UUID: {}", e)))?;
            let old_safety: i64 = r.get("old_safety_stock");
            let new_safety: i64 = r.get("new_safety_stock");
            let admin_note: String = r.get("admin_note");
            let updated_by: String = r.get("updated_by");
            let timestamp_str: String = r.get("timestamp");
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            logs.push(SafetyStockLogDto {
                id,
                product_id,
                old_safety_stock: old_safety as u32,
                new_safety_stock: new_safety as u32,
                admin_note,
                updated_by,
                timestamp,
            });
        }

        Ok(logs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use program1_core::init_database;
    use program1_module_catalog::CatalogModule;

    #[tokio::test]
    async fn test_ginee_multi_stock_and_safety_logs() {
        let pool = init_database("sqlite::memory:").await.unwrap();
        let catalog = Arc::new(CatalogModule::new(pool.clone()));
        catalog.seed_default_catalog().await.unwrap();
        let inventory = InventoryModule::new(pool, catalog.clone());

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
