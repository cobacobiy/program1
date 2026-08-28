use async_trait::async_trait;
use chrono::{DateTime, Utc};
use program1_contracts::{
    CatalogContract, CatalogItemDto, ContractError, CreateCatalogItemRequest,
};
use program1_core::database::DbPool;
use sqlx::Row;
use uuid::Uuid;

#[derive(Clone)]
pub struct CatalogModule {
    pool: DbPool,
}

impl CatalogModule {
    pub fn new(pool: DbPool) -> Self {
        let module = Self { pool };
        let module_clone = module.clone();
        tokio::spawn(async move {
            let _ = module_clone.seed_default_catalog().await;
        });
        module
    }

    pub async fn seed_default_catalog(&self) -> Result<(), ContractError> {
        let count_row = sqlx::query("SELECT COUNT(*) as count FROM catalog_items")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let count: i64 = count_row.get("count");
        if count > 0 {
            return Ok(());
        }

        let initial_items = vec![
            (
                "10000000-0000-0000-0000-000000000001",
                "AURA Wireless Mechanical Keyboard",
                "SKU-AURA-KB01",
                "Peripherals",
                1450000.0,
                45,
                "https://images.unsplash.com/photo-1587829741301-dc798b83add3?auto=format&fit=crop&w=500&q=80",
                "RGB Hotswap Gasket Mount Keyboard with Bluetooth 5.2 and 2.4GHz Receiver.",
            ),
            (
                "10000000-0000-0000-0000-000000000002",
                "AURA Ergonomic Precision Mouse",
                "SKU-AURA-MS02",
                "Peripherals",
                780000.0,
                80,
                "https://images.unsplash.com/photo-1615663245857-ac93bb7c39e7?auto=format&fit=crop&w=500&q=80",
                "26K DPI Optical Sensor with Dual Wireless & Type-C Charging Dock.",
            ),
            (
                "10000000-0000-0000-0000-000000000003",
                "AURA Ultra-Wide Glass Monitor Arm",
                "SKU-AURA-ARM03",
                "Accessories",
                950000.0,
                30,
                "https://images.unsplash.com/photo-1527443224154-c4a3942d3acf?auto=format&fit=crop&w=500&q=80",
                "Heavy Duty Gas Spring Arm supporting up to 49-inch Ultrawide Displays.",
            ),
        ];

        for (id, name, sku, category, price, stock, img, desc) in initial_items {
            let _ = sqlx::query(
                "INSERT OR IGNORE INTO catalog_items (id, name, sku, category, price, stock, image_url, description, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            )
            .bind(id)
            .bind(name)
            .bind(sku)
            .bind(category)
            .bind(price)
            .bind(stock)
            .bind(img)
            .bind(desc)
            .bind(Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await;
        }

        Ok(())
    }

    fn row_to_dto(row: &sqlx::sqlite::SqliteRow) -> Result<CatalogItemDto, ContractError> {
        let id_str: String = row.get("id");
        let id = Uuid::parse_str(&id_str)
            .map_err(|e| ContractError::Internal(format!("Corrupt UUID in catalog: {}", e)))?;

        let name: String = row.get("name");
        let sku: String = row.get("sku");
        let category: String = row.get("category");
        let price: f64 = row.get("price");
        let stock: i64 = row.get("stock");
        let image_url: String = row.get("image_url");
        let description: String = row.get("description");
        let created_at_str: String = row.get("created_at");
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(CatalogItemDto {
            id,
            name,
            sku,
            category,
            price,
            stock: stock as u32,
            image_url,
            description,
            created_at,
        })
    }
}

#[async_trait]
impl CatalogContract for CatalogModule {
    async fn list_items(&self) -> Result<Vec<CatalogItemDto>, ContractError> {
        let rows = sqlx::query(
            "SELECT id, name, sku, category, price, stock, image_url, description, created_at
             FROM catalog_items ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        rows.iter().map(Self::row_to_dto).collect()
    }

    async fn get_item(&self, id: Uuid) -> Result<CatalogItemDto, ContractError> {
        let row = sqlx::query(
            "SELECT id, name, sku, category, price, stock, image_url, description, created_at
             FROM catalog_items WHERE id = $1",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        match row {
            Some(r) => Self::row_to_dto(&r),
            None => Err(ContractError::NotFound(format!("Catalog Item {}", id))),
        }
    }

    async fn create_item(&self, req: CreateCatalogItemRequest) -> Result<CatalogItemDto, ContractError> {
        if req.name.trim().is_empty() {
            return Err(ContractError::ValidationError("Item name cannot be empty".to_string()));
        }
        if req.price < 0.0 {
            return Err(ContractError::ValidationError("Price cannot be negative".to_string()));
        }

        let id = Uuid::new_v4();
        let sku = req.sku.trim().to_uppercase();
        let name = req.name.trim().to_string();
        let category = req.category.trim().to_string();
        let image_url = req.image_url.unwrap_or_else(|| "https://via.placeholder.com/500".to_string());
        let description = req.description.unwrap_or_else(|| "Product description".to_string());
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO catalog_items (id, name, sku, category, price, stock, image_url, description, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id.to_string())
        .bind(&name)
        .bind(&sku)
        .bind(&category)
        .bind(req.price)
        .bind(req.stock as i64)
        .bind(&image_url)
        .bind(&description)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        tracing::info!(id = %id, sku = %sku, "Catalog item created in database");

        Ok(CatalogItemDto {
            id,
            name,
            sku,
            category,
            price: req.price,
            stock: req.stock,
            image_url,
            description,
            created_at: now,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use program1_core::init_database;

    async fn create_test_catalog_module() -> CatalogModule {
        let pool = init_database("sqlite::memory:").await.expect("In-memory SQLite init failed");
        let module = CatalogModule::new(pool);
        module.seed_default_catalog().await.expect("Seeding failed");
        module
    }

    #[tokio::test]
    async fn test_catalog_crud() {
        let module = create_test_catalog_module().await;
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
