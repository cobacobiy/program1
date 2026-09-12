use async_trait::async_trait;
use chrono::{DateTime, Utc};
use program1_contracts::{
    AddFlashSaleItemRequest, ContractError, CreateFlashSaleSessionRequest, FlashSaleContract,
    FlashSaleItemDto, FlashSaleSessionDto,
};
use program1_core::database::DbPool;
use sqlx::Row;
use uuid::Uuid;

#[derive(Clone)]
pub struct FlashSaleModule {
    pool: DbPool,
}

impl FlashSaleModule {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    async fn fetch_items_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<FlashSaleItemDto>, ContractError> {
        let rows = sqlx::query(
            "SELECT f.id, f.session_id, f.product_id, f.flash_price_cents, f.allocated_quantity, f.sold_quantity,
                    c.name as product_name, c.price as original_price, c.image_url
             FROM flash_sale_items f
             JOIN catalog_items c ON c.id = f.product_id
             WHERE f.session_id = $1",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let mut items = Vec::new();
        for r in rows {
            let original_price: f64 = r.get("original_price");
            let img: String = r.get("image_url");

            items.push(FlashSaleItemDto {
                id: r.get("id"),
                session_id: r.get("session_id"),
                product_id: r.get("product_id"),
                product_name: r.get("product_name"),
                original_price_cents: original_price as i64,
                flash_price_cents: r.get("flash_price_cents"),
                allocated_quantity: r.get("allocated_quantity"),
                sold_quantity: r.get("sold_quantity"),
                image_url: if img.is_empty() { None } else { Some(img) },
            });
        }
        Ok(items)
    }

    fn calculate_remaining_seconds(end_time_str: &str) -> Option<i64> {
        DateTime::parse_from_rfc3339(end_time_str)
            .map(|dt| {
                let end = dt.with_timezone(&Utc);
                let now = Utc::now();
                (end - now).num_seconds().max(0)
            })
            .ok()
    }
}

#[async_trait]
impl FlashSaleContract for FlashSaleModule {
    async fn get_active_session(&self) -> Result<Option<FlashSaleSessionDto>, ContractError> {
        let now_str = Utc::now().to_rfc3339();

        let row = sqlx::query(
            "SELECT id, title, start_time, end_time, banner_url, is_active
             FROM flash_sale_sessions
             WHERE is_active = 1 AND start_time <= $1 AND end_time >= $1
             ORDER BY start_time ASC
             LIMIT 1",
        )
        .bind(&now_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let row = match row {
            Some(r) => r,
            None => return Ok(None),
        };

        let session_id: String = row.get("id");
        let title: String = row.get("title");
        let start_time: String = row.get("start_time");
        let end_time: String = row.get("end_time");
        let banner_url: Option<String> = row.try_get("banner_url").ok().flatten();
        let is_active_int: i64 = row.get("is_active");

        let items = self.fetch_items_for_session(&session_id).await?;
        let remaining_seconds = Self::calculate_remaining_seconds(&end_time);

        Ok(Some(FlashSaleSessionDto {
            id: session_id,
            title,
            start_time,
            end_time,
            banner_url,
            is_active: is_active_int != 0,
            items,
            remaining_seconds,
        }))
    }

    async fn list_sessions(&self) -> Result<Vec<FlashSaleSessionDto>, ContractError> {
        let rows = sqlx::query(
            "SELECT id, title, start_time, end_time, banner_url, is_active
             FROM flash_sale_sessions
             ORDER BY start_time DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let mut list = Vec::new();
        for r in rows {
            let session_id: String = r.get("id");
            let title: String = r.get("title");
            let start_time: String = r.get("start_time");
            let end_time: String = r.get("end_time");
            let banner_url: Option<String> = r.try_get("banner_url").ok().flatten();
            let is_active_int: i64 = r.get("is_active");

            let items = self.fetch_items_for_session(&session_id).await?;
            let remaining_seconds = Self::calculate_remaining_seconds(&end_time);

            list.push(FlashSaleSessionDto {
                id: session_id,
                title,
                start_time,
                end_time,
                banner_url,
                is_active: is_active_int != 0,
                items,
                remaining_seconds,
            });
        }
        Ok(list)
    }

    async fn get_session(&self, session_id: &str) -> Result<FlashSaleSessionDto, ContractError> {
        let row = sqlx::query(
            "SELECT id, title, start_time, end_time, banner_url, is_active
             FROM flash_sale_sessions
             WHERE id = $1",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let row = match row {
            Some(r) => r,
            None => {
                return Err(ContractError::NotFound(format!(
                    "Flash sale session {}",
                    session_id
                )))
            }
        };

        let session_id: String = row.get("id");
        let title: String = row.get("title");
        let start_time: String = row.get("start_time");
        let end_time: String = row.get("end_time");
        let banner_url: Option<String> = row.try_get("banner_url").ok().flatten();
        let is_active_int: i64 = row.get("is_active");

        let items = self.fetch_items_for_session(&session_id).await?;
        let remaining_seconds = Self::calculate_remaining_seconds(&end_time);

        Ok(FlashSaleSessionDto {
            id: session_id,
            title,
            start_time,
            end_time,
            banner_url,
            is_active: is_active_int != 0,
            items,
            remaining_seconds,
        })
    }

    async fn create_session(
        &self,
        req: CreateFlashSaleSessionRequest,
    ) -> Result<FlashSaleSessionDto, ContractError> {
        let title = req.title.trim().to_string();
        if title.is_empty() {
            return Err(ContractError::ValidationError(
                "Judul flash sale tidak boleh kosong".to_string(),
            ));
        }

        let start_dt = DateTime::parse_from_rfc3339(&req.start_time).map_err(|_| {
            ContractError::ValidationError("Format waktu start_time harus RFC3339".to_string())
        })?;
        let end_dt = DateTime::parse_from_rfc3339(&req.end_time).map_err(|_| {
            ContractError::ValidationError("Format waktu end_time harus RFC3339".to_string())
        })?;

        if start_dt >= end_dt {
            return Err(ContractError::ValidationError(
                "Waktu mulai harus sebelum waktu berakhir".to_string(),
            ));
        }

        let id = Uuid::new_v4().to_string();
        let start_str = start_dt.to_rfc3339();
        let end_str = end_dt.to_rfc3339();

        sqlx::query(
            "INSERT INTO flash_sale_sessions (id, title, start_time, end_time, banner_url, is_active, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, 1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        )
        .bind(&id)
        .bind(&title)
        .bind(&start_str)
        .bind(&end_str)
        .bind(&req.banner_url)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let remaining_seconds = Self::calculate_remaining_seconds(&end_str);

        Ok(FlashSaleSessionDto {
            id,
            title,
            start_time: start_str,
            end_time: end_str,
            banner_url: req.banner_url,
            is_active: true,
            items: vec![],
            remaining_seconds,
        })
    }

    async fn add_item_to_session(
        &self,
        session_id: &str,
        req: AddFlashSaleItemRequest,
    ) -> Result<FlashSaleItemDto, ContractError> {
        let _ = self.get_session(session_id).await?;

        // Check if product exists in catalog
        let prod_row = sqlx::query("SELECT name, price, image_url FROM catalog_items WHERE id = $1")
            .bind(&req.product_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let prod_row = match prod_row {
            Some(r) => r,
            None => {
                return Err(ContractError::NotFound(format!(
                    "Product {}",
                    req.product_id
                )))
            }
        };

        let product_name: String = prod_row.get("name");
        let original_price: f64 = prod_row.get("price");
        let image_url: String = prod_row.get("image_url");

        let item_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO flash_sale_items (id, session_id, product_id, flash_price_cents, allocated_quantity, sold_quantity, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, 0, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
             ON CONFLICT(session_id, product_id) DO UPDATE SET
               flash_price_cents = excluded.flash_price_cents,
               allocated_quantity = excluded.allocated_quantity,
               updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(&item_id)
        .bind(session_id)
        .bind(&req.product_id)
        .bind(req.flash_price_cents)
        .bind(req.allocated_quantity)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        Ok(FlashSaleItemDto {
            id: item_id,
            session_id: session_id.to_string(),
            product_id: req.product_id,
            product_name,
            original_price_cents: original_price as i64,
            flash_price_cents: req.flash_price_cents,
            allocated_quantity: req.allocated_quantity,
            sold_quantity: 0,
            image_url: if image_url.is_empty() {
                None
            } else {
                Some(image_url)
            },
        })
    }

    async fn toggle_session(&self, session_id: &str) -> Result<FlashSaleSessionDto, ContractError> {
        let current = self.get_session(session_id).await?;
        let new_active = !current.is_active;

        sqlx::query(
            "UPDATE flash_sale_sessions SET is_active = $1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = $2",
        )
        .bind(if new_active { 1 } else { 0 })
        .bind(session_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        self.get_session(session_id).await
    }

    async fn check_flash_price(&self, product_id: &str) -> Result<Option<i64>, ContractError> {
        let now_str = Utc::now().to_rfc3339();

        let row = sqlx::query(
            "SELECT f.flash_price_cents
             FROM flash_sale_items f
             JOIN flash_sale_sessions s ON s.id = f.session_id
             WHERE f.product_id = $1
               AND s.is_active = 1
               AND s.start_time <= $2
               AND s.end_time >= $2
               AND f.allocated_quantity > f.sold_quantity
             LIMIT 1",
        )
        .bind(product_id)
        .bind(&now_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        Ok(row.map(|r| r.get("flash_price_cents")))
    }

    async fn record_flash_sale_purchase(
        &self,
        product_id: &str,
        quantity: i32,
    ) -> Result<bool, ContractError> {
        let now_str = Utc::now().to_rfc3339();

        let res = sqlx::query(
            "UPDATE flash_sale_items SET sold_quantity = sold_quantity + $1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE product_id = $2
               AND session_id IN (
                   SELECT id FROM flash_sale_sessions
                   WHERE is_active = 1 AND start_time <= $3 AND end_time >= $3
               )
               AND (allocated_quantity - sold_quantity) >= $1",
        )
        .bind(quantity)
        .bind(product_id)
        .bind(&now_str)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        Ok(res.rows_affected() > 0)
    }
}
