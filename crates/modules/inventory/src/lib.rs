use std::sync::Arc;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use program1_contracts::{
    BulkStockUpdateRequest, BulkStockUpdateResult, CatalogContract, ContractError,
    InventoryContract, InventoryStockDto, LowStockAlertDto, SafetyStockLogDto,
    StockAdjustmentLogDto,
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

    async fn insert_adjustment_log(
        &self,
        product_id: Uuid,
        adjustment_type: &str,
        old_value: u32,
        new_value: u32,
        admin_note: &str,
        updated_by: &str,
    ) -> Result<(), ContractError> {
        let log_id = Uuid::new_v4();
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO stock_adjustment_logs (id, product_id, adjustment_type, old_value, new_value, admin_note, updated_by, timestamp)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(log_id.to_string())
        .bind(product_id.to_string())
        .bind(adjustment_type)
        .bind(old_value as i64)
        .bind(new_value as i64)
        .bind(admin_note.trim())
        .bind(updated_by.trim())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;
        Ok(())
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

        // Insert legacy audit log
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

        // Insert unified adjustment log
        let _ = self
            .insert_adjustment_log(
                product_id,
                "safety",
                old_safety_stock,
                new_safety_stock,
                &admin_note,
                &operator,
            )
            .await;

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

    async fn update_warehouse_stock(
        &self,
        product_id: Uuid,
        new_warehouse_stock: u32,
        admin_note: String,
        updated_by: String,
    ) -> Result<InventoryStockDto, ContractError> {
        if admin_note.trim().is_empty() {
            return Err(ContractError::ValidationError(
                "Catatan Admin (Alasan Perubahan Stok Gudang) wajib diisi!".to_string(),
            ));
        }

        let stock = self.ensure_product_initialized(product_id).await?;
        let old_warehouse_stock = stock.warehouse_stock;
        let now = Utc::now();
        let operator = if updated_by.trim().is_empty() {
            "Admin Ginee".to_string()
        } else {
            updated_by.trim().to_string()
        };

        let total_allocated = stock.locked_stock + stock.spare_stock + stock.promotion_stock + stock.safety_stock;
        if new_warehouse_stock < total_allocated {
            return Err(ContractError::ValidationError(
                format!(
                    "Stok gudang baru ({}) tidak boleh lebih kecil dari total stok teralokasi (locked + spare + promo + safety = {})",
                    new_warehouse_stock, total_allocated
                ),
            ));
        }

        let new_available = Self::calculate_available(
            new_warehouse_stock,
            stock.locked_stock,
            stock.spare_stock,
            stock.promotion_stock,
            stock.safety_stock,
        );

        sqlx::query(
            "UPDATE inventory_stocks
             SET warehouse_stock = $1, available_stock = $2, last_updated = $3
             WHERE product_id = $4",
        )
        .bind(new_warehouse_stock as i64)
        .bind(new_available as i64)
        .bind(now.to_rfc3339())
        .bind(product_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let _ = self
            .insert_adjustment_log(
                product_id,
                "warehouse",
                old_warehouse_stock,
                new_warehouse_stock,
                &admin_note,
                &operator,
            )
            .await;

        tracing::info!(
            product_id = %product_id,
            old = old_warehouse_stock,
            new = new_warehouse_stock,
            note = %admin_note,
            "Warehouse stock updated in database"
        );

        self.get_stock(product_id).await
    }

    async fn update_spare_stock(
        &self,
        product_id: Uuid,
        new_spare_stock: u32,
        admin_note: String,
        updated_by: String,
    ) -> Result<InventoryStockDto, ContractError> {
        if admin_note.trim().is_empty() {
            return Err(ContractError::ValidationError(
                "Catatan Admin (Alasan Perubahan Stok Cadangan) wajib diisi!".to_string(),
            ));
        }

        let stock = self.ensure_product_initialized(product_id).await?;
        let old_spare_stock = stock.spare_stock;
        let now = Utc::now();
        let operator = if updated_by.trim().is_empty() {
            "Admin Ginee".to_string()
        } else {
            updated_by.trim().to_string()
        };

        let total_allocated = stock.locked_stock + new_spare_stock + stock.promotion_stock + stock.safety_stock;
        if total_allocated > stock.warehouse_stock {
            return Err(ContractError::ValidationError(
                format!(
                    "Total stok teralokasi ({}) melebihi stok gudang ({})",
                    total_allocated, stock.warehouse_stock
                ),
            ));
        }

        let new_available = Self::calculate_available(
            stock.warehouse_stock,
            stock.locked_stock,
            new_spare_stock,
            stock.promotion_stock,
            stock.safety_stock,
        );

        sqlx::query(
            "UPDATE inventory_stocks
             SET spare_stock = $1, available_stock = $2, last_updated = $3
             WHERE product_id = $4",
        )
        .bind(new_spare_stock as i64)
        .bind(new_available as i64)
        .bind(now.to_rfc3339())
        .bind(product_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let _ = self
            .insert_adjustment_log(
                product_id,
                "spare",
                old_spare_stock,
                new_spare_stock,
                &admin_note,
                &operator,
            )
            .await;

        tracing::info!(
            product_id = %product_id,
            old = old_spare_stock,
            new = new_spare_stock,
            note = %admin_note,
            "Spare stock updated in database"
        );

        self.get_stock(product_id).await
    }

    async fn update_promotion_stock(
        &self,
        product_id: Uuid,
        new_promotion_stock: u32,
        admin_note: String,
        updated_by: String,
    ) -> Result<InventoryStockDto, ContractError> {
        if admin_note.trim().is_empty() {
            return Err(ContractError::ValidationError(
                "Catatan Admin (Alasan Perubahan Stok Promosi) wajib diisi!".to_string(),
            ));
        }

        let stock = self.ensure_product_initialized(product_id).await?;
        let old_promotion_stock = stock.promotion_stock;
        let now = Utc::now();
        let operator = if updated_by.trim().is_empty() {
            "Admin Ginee".to_string()
        } else {
            updated_by.trim().to_string()
        };

        let total_allocated = stock.locked_stock + stock.spare_stock + new_promotion_stock + stock.safety_stock;
        if total_allocated > stock.warehouse_stock {
            return Err(ContractError::ValidationError(
                format!(
                    "Total stok teralokasi ({}) melebihi stok gudang ({})",
                    total_allocated, stock.warehouse_stock
                ),
            ));
        }

        let new_available = Self::calculate_available(
            stock.warehouse_stock,
            stock.locked_stock,
            stock.spare_stock,
            new_promotion_stock,
            stock.safety_stock,
        );

        sqlx::query(
            "UPDATE inventory_stocks
             SET promotion_stock = $1, available_stock = $2, last_updated = $3
             WHERE product_id = $4",
        )
        .bind(new_promotion_stock as i64)
        .bind(new_available as i64)
        .bind(now.to_rfc3339())
        .bind(product_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let _ = self
            .insert_adjustment_log(
                product_id,
                "promotion",
                old_promotion_stock,
                new_promotion_stock,
                &admin_note,
                &operator,
            )
            .await;

        tracing::info!(
            product_id = %product_id,
            old = old_promotion_stock,
            new = new_promotion_stock,
            note = %admin_note,
            "Promotion stock updated in database"
        );

        self.get_stock(product_id).await
    }

    async fn get_adjustment_logs(
        &self,
        product_id: Uuid,
        adjustment_type: Option<&str>,
    ) -> Result<Vec<StockAdjustmentLogDto>, ContractError> {
        let rows = if let Some(adj_type) = adjustment_type {
            sqlx::query(
                "SELECT id, product_id, adjustment_type, old_value, new_value, admin_note, updated_by, timestamp
                 FROM stock_adjustment_logs WHERE product_id = $1 AND adjustment_type = $2 ORDER BY timestamp DESC",
            )
            .bind(product_id.to_string())
            .bind(adj_type)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?
        } else {
            sqlx::query(
                "SELECT id, product_id, adjustment_type, old_value, new_value, admin_note, updated_by, timestamp
                 FROM stock_adjustment_logs WHERE product_id = $1 ORDER BY timestamp DESC",
            )
            .bind(product_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?
        };

        let mut logs = Vec::new();
        for r in rows {
            let id_str: String = r.get("id");
            let id = Uuid::parse_str(&id_str)
                .map_err(|e| ContractError::Internal(format!("Invalid UUID: {}", e)))?;
            let adj_type: String = r.get("adjustment_type");
            let old_value: i64 = r.get("old_value");
            let new_value: i64 = r.get("new_value");
            let admin_note: String = r.get("admin_note");
            let updated_by: String = r.get("updated_by");
            let timestamp_str: String = r.get("timestamp");
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            logs.push(StockAdjustmentLogDto {
                id,
                product_id,
                adjustment_type: adj_type,
                old_value: old_value as u32,
                new_value: new_value as u32,
                admin_note,
                updated_by,
                timestamp,
            });
        }

        Ok(logs)
    }

    async fn get_low_stock_alerts(&self) -> Result<Vec<LowStockAlertDto>, ContractError> {
        let all_stocks = self.get_all_stocks().await?;
        let mut alerts = Vec::new();

        for stock in all_stocks {
            if stock.available_stock <= stock.safety_stock {
                let deficit = stock.safety_stock.saturating_sub(stock.available_stock);
                let percentage = if stock.safety_stock > 0 {
                    (stock.available_stock as f64 / stock.safety_stock as f64) * 100.0
                } else {
                    0.0
                };

                let severity = if stock.available_stock == 0 || percentage < 50.0 {
                    "critical"
                } else if percentage < 100.0 {
                    "warning"
                } else {
                    "caution"
                };

                alerts.push(LowStockAlertDto {
                    product_id: stock.product_id,
                    product_name: stock.product_name,
                    sku: stock.sku,
                    available_stock: stock.available_stock,
                    safety_stock: stock.safety_stock,
                    deficit,
                    severity: severity.to_string(),
                });
            }
        }

        alerts.sort_by(|a, b| {
            let score_a = match a.severity.as_str() {
                "critical" => 0,
                "warning" => 1,
                _ => 2,
            };
            let score_b = match b.severity.as_str() {
                "critical" => 0,
                "warning" => 1,
                _ => 2,
            };
            score_a.cmp(&score_b).then_with(|| b.deficit.cmp(&a.deficit))
        });

        Ok(alerts)
    }

    async fn bulk_update_stock(
        &self,
        request: BulkStockUpdateRequest,
    ) -> Result<BulkStockUpdateResult, ContractError> {
        let operator = request
            .updated_by
            .unwrap_or_else(|| "Admin Ginee".to_string());
        let mut success = 0u32;
        let mut failed = 0u32;
        let mut errors = Vec::new();

        for item in &request.adjustments {
            let result = match item.stock_type.to_lowercase().as_str() {
                "warehouse" => {
                    self.update_warehouse_stock(
                        item.product_id,
                        item.new_value,
                        request.admin_note.clone(),
                        operator.clone(),
                    )
                    .await
                }
                "safety" => {
                    self.update_safety_stock(
                        item.product_id,
                        item.new_value,
                        request.admin_note.clone(),
                        operator.clone(),
                    )
                    .await
                }
                "spare" => {
                    self.update_spare_stock(
                        item.product_id,
                        item.new_value,
                        request.admin_note.clone(),
                        operator.clone(),
                    )
                    .await
                }
                "promotion" => {
                    self.update_promotion_stock(
                        item.product_id,
                        item.new_value,
                        request.admin_note.clone(),
                        operator.clone(),
                    )
                    .await
                }
                other => Err(ContractError::ValidationError(format!(
                    "Tipe stok '{}' tidak dikenali (gunakan warehouse/safety/spare/promotion)",
                    other
                ))),
            };

            match result {
                Ok(_) => success += 1,
                Err(e) => {
                    failed += 1;
                    errors.push(format!("{}: {}", item.product_id, e));
                    tracing::warn!(
                        product_id = %item.product_id,
                        stock_type = %item.stock_type,
                        error = %e,
                        "Bulk stock adjustment item failed"
                    );
                }
            }
        }

        if failed > 0 {
            tracing::warn!(
                total = request.adjustments.len(),
                success = success,
                failed = failed,
                "Bulk stock update completed with partial failures"
            );
        }

        Ok(BulkStockUpdateResult {
            total_requested: request.adjustments.len() as u32,
            total_success: success,
            total_failed: failed,
            errors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use program1_contracts::BulkStockAdjustmentItem;
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

    #[tokio::test]
    async fn test_multi_stock_adjustments_and_unified_logs() {
        let pool = init_database("sqlite::memory:").await.unwrap();
        let catalog = Arc::new(CatalogModule::new(pool.clone()));
        catalog.seed_default_catalog().await.unwrap();
        let inventory = InventoryModule::new(pool, catalog.clone());

        let items = catalog.list_items().await.unwrap();
        let target = &items[0];

        // 1. Update warehouse stock
        let updated_warehouse = inventory
            .update_warehouse_stock(target.id, 1000, "Stock opname gudang A".to_string(), "Admin Gudang".to_string())
            .await
            .unwrap();
        assert_eq!(updated_warehouse.warehouse_stock, 1000);

        // 2. Update spare stock
        let updated_spare = inventory
            .update_spare_stock(target.id, 25, "Alokasi sampel event".to_string(), "Admin Gudang".to_string())
            .await
            .unwrap();
        assert_eq!(updated_spare.spare_stock, 25);

        // 3. Update promotion stock
        let updated_promo = inventory
            .update_promotion_stock(target.id, 100, "Flash sale 11.11".to_string(), "Admin Promo".to_string())
            .await
            .unwrap();
        assert_eq!(updated_promo.promotion_stock, 100);

        // 4. Check unified logs
        let all_logs = inventory.get_adjustment_logs(target.id, None).await.unwrap();
        assert_eq!(all_logs.len(), 3);

        let promo_logs = inventory.get_adjustment_logs(target.id, Some("promotion")).await.unwrap();
        assert_eq!(promo_logs.len(), 1);
        assert_eq!(promo_logs[0].old_value, 0);
        assert_eq!(promo_logs[0].new_value, 100);
    }

    #[tokio::test]
    async fn test_low_stock_alerts_and_bulk_update() {
        let pool = init_database("sqlite::memory:").await.unwrap();
        let catalog = Arc::new(CatalogModule::new(pool.clone()));
        catalog.seed_default_catalog().await.unwrap();
        let inventory = InventoryModule::new(pool, catalog.clone());

        let items = catalog.list_items().await.unwrap();
        let target1 = &items[0];
        let target2 = &items[1];

        // Bulk update: set high safety stock to trigger alert
        let bulk_req = BulkStockUpdateRequest {
            adjustments: vec![
                BulkStockAdjustmentItem {
                    product_id: target1.id,
                    stock_type: "safety".to_string(),
                    new_value: 600, // Available will be < safety
                },
                BulkStockAdjustmentItem {
                    product_id: target2.id,
                    stock_type: "spare".to_string(),
                    new_value: 30,
                },
            ],
            admin_note: "Bulk seasonal adjustment".to_string(),
            updated_by: Some("SuperAdmin".to_string()),
        };

        let bulk_res = inventory.bulk_update_stock(bulk_req).await.unwrap();
        assert_eq!(bulk_res.total_requested, 2);
        assert_eq!(bulk_res.total_success, 2);
        assert_eq!(bulk_res.total_failed, 0);

        // Check low stock alerts
        let alerts = inventory.get_low_stock_alerts().await.unwrap();
        assert!(!alerts.is_empty());
        assert!(alerts.iter().any(|a| a.product_id == target1.id));
    }
}

