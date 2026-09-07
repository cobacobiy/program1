use async_trait::async_trait;
use chrono::{DateTime, Utc};
use program1_contracts::{
    ContractError, CreateReviewRequest, PaginatedResponse, ProductRatingSummaryDto,
    ProductReviewDto, PublicReviewDto, ReviewContract,
};
use program1_core::database::DbPool;
use sqlx::Row;
use uuid::Uuid;

#[derive(Clone)]
pub struct ReviewModule {
    pool: DbPool,
}

impl ReviewModule {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    fn parse_datetime(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|_| {
                chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%fZ")
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
            })
            .or_else(|_| {
                chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
            })
            .unwrap_or_else(|_| Utc::now())
    }

    fn row_to_dto(row: &sqlx::sqlite::SqliteRow) -> Result<ProductReviewDto, ContractError> {
        let id_str: String = row.get("id");
        let id = Uuid::parse_str(&id_str)
            .map_err(|e| ContractError::Internal(format!("Corrupt review id UUID: {}", e)))?;

        let prod_id_str: String = row.get("product_id");
        let product_id = Uuid::parse_str(&prod_id_str)
            .map_err(|e| ContractError::Internal(format!("Corrupt product_id UUID: {}", e)))?;

        let buyer_id_str: String = row.get("buyer_id");
        let buyer_id = Uuid::parse_str(&buyer_id_str)
            .map_err(|e| ContractError::Internal(format!("Corrupt buyer_id UUID: {}", e)))?;

        let buyer_name: String = row
            .try_get("buyer_name")
            .unwrap_or_else(|_| "Pembeli".to_string());

        let order_id_str: String = row.get("order_id");
        let order_id = Uuid::parse_str(&order_id_str)
            .map_err(|e| ContractError::Internal(format!("Corrupt order_id UUID: {}", e)))?;

        let rating: i64 = row.get("rating");
        let review_text: Option<String> = row.get("review_text");
        let is_visible_int: i64 = row.get("is_visible");
        let is_visible = is_visible_int != 0;

        let created_at_str: String = row.get("created_at");
        let created_at = Self::parse_datetime(&created_at_str);

        let updated_at_str: String = row.get("updated_at");
        let updated_at = Self::parse_datetime(&updated_at_str);

        Ok(ProductReviewDto {
            id,
            product_id,
            buyer_id,
            buyer_name,
            order_id,
            rating: rating as i32,
            review_text,
            is_visible,
            created_at,
            updated_at,
        })
    }

    fn row_to_public_dto(row: &sqlx::sqlite::SqliteRow) -> Result<PublicReviewDto, ContractError> {
        let id_str: String = row.get("id");
        let id = Uuid::parse_str(&id_str)
            .map_err(|e| ContractError::Internal(format!("Corrupt review UUID: {}", e)))?;

        let product_id_str: String = row.get("product_id");
        let product_id = Uuid::parse_str(&product_id_str)
            .map_err(|e| ContractError::Internal(format!("Corrupt product UUID: {}", e)))?;

        let buyer_name: String = row.get("buyer_name");

        let rating: i64 = row.get("rating");
        let review_text: Option<String> = row.get("review_text");

        let created_at_str: String = row.get("created_at");
        let created_at = Self::parse_datetime(&created_at_str);

        let updated_at_str: String = row.get("updated_at");
        let updated_at = Self::parse_datetime(&updated_at_str);

        Ok(PublicReviewDto {
            id,
            product_id,
            buyer_name,
            rating: rating as i32,
            review_text,
            created_at,
            updated_at,
        })
    }
}

#[async_trait]
impl ReviewContract for ReviewModule {
    async fn create_review(
        &self,
        buyer_id: Uuid,
        req: CreateReviewRequest,
    ) -> Result<ProductReviewDto, ContractError> {
        // 1. Validate rating bounds
        if req.rating < 1 || req.rating > 5 {
            return Err(ContractError::ValidationError(
                "Rating must be between 1 and 5".to_string(),
            ));
        }

        let buyer_id_str = buyer_id.to_string();
        let order_id_str = req.order_id.to_string();
        let product_id_str = req.product_id.to_string();

        // 2. Fetch order to verify existence and ownership
        let order_row = sqlx::query("SELECT buyer_id, status FROM orders WHERE id = $1")
            .bind(&order_id_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let (order_buyer_id, order_status): (Option<String>, String) = match order_row {
            Some(row) => (row.get("buyer_id"), row.get("status")),
            None => return Err(ContractError::NotFound("Order not found".to_string())),
        };

        // 3. Verify buyer ownership (IDOR & cross-buyer protection)
        if order_buyer_id.as_deref() != Some(&buyer_id_str) {
            return Err(ContractError::ValidationError("Not your order".to_string()));
        }

        // 4. Verify order status is delivered
        if !order_status.eq_ignore_ascii_case("delivered")
            && !order_status.eq_ignore_ascii_case("completed")
        {
            return Err(ContractError::ValidationError(
                "Can only review delivered orders".to_string(),
            ));
        }

        // 5. Verify product is actually in this order
        let item_row = sqlx::query(
            "SELECT COUNT(*) as cnt FROM order_items WHERE order_id = $1 AND product_id = $2",
        )
        .bind(&order_id_str)
        .bind(&product_id_str)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let item_count: i64 = item_row.get("cnt");
        if item_count == 0 {
            return Err(ContractError::ValidationError(
                "Product not found in this order".to_string(),
            ));
        }

        // 6. Check existing review at service layer (prevent duplicate review)
        let existing_review = sqlx::query(
            "SELECT COUNT(*) as cnt FROM product_reviews WHERE product_id = $1 AND buyer_id = $2 AND order_id = $3",
        )
        .bind(&product_id_str)
        .bind(&buyer_id_str)
        .bind(&order_id_str)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let existing_count: i64 = existing_review.get("cnt");
        if existing_count > 0 {
            return Err(ContractError::ValidationError(
                "You have already reviewed this product for this order".to_string(),
            ));
        }

        // 7. Sanitize review text (strip HTML, trim, max 1000 chars)
        let sanitized_text = req
            .review_text
            .as_deref()
            .map(|t| program1_core::sanitize::sanitize_text(t, 1000))
            .filter(|t| !t.trim().is_empty());

        let review_id = Uuid::new_v4();
        let review_id_str = review_id.to_string();
        let now_str = Utc::now().to_rfc3339();

        // 8. Insert review into database (unique constraint enforces race safety)
        let insert_res = sqlx::query(
            r#"
            INSERT INTO product_reviews (id, product_id, buyer_id, order_id, rating, review_text, is_visible, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, 1, $7, $8)
            "#,
        )
        .bind(&review_id_str)
        .bind(&product_id_str)
        .bind(&buyer_id_str)
        .bind(&order_id_str)
        .bind(req.rating)
        .bind(&sanitized_text)
        .bind(&now_str)
        .bind(&now_str)
        .execute(&self.pool)
        .await;

        if let Err(e) = insert_res {
            let err_msg = e.to_string();
            if err_msg.contains("UNIQUE") || err_msg.contains("unique") {
                return Err(ContractError::ValidationError(
                    "You have already reviewed this product for this order".to_string(),
                ));
            }
            return Err(ContractError::Internal(err_msg));
        }

        // 9. Fetch buyer display name (no email/phone PII)
        let buyer_row = sqlx::query("SELECT full_name FROM buyer_accounts WHERE id = $1")
            .bind(&buyer_id_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let buyer_name = buyer_row
            .map(|r| r.get::<String, _>("full_name"))
            .unwrap_or_else(|| "Pembeli".to_string());

        let now_dt = Self::parse_datetime(&now_str);

        Ok(ProductReviewDto {
            id: review_id,
            product_id: req.product_id,
            buyer_id,
            buyer_name,
            order_id: req.order_id,
            rating: req.rating,
            review_text: sanitized_text,
            is_visible: true,
            created_at: now_dt,
            updated_at: now_dt,
        })
    }

    async fn get_reviews_for_product(
        &self,
        product_id: Uuid,
        page: i64,
        page_size: i64,
    ) -> Result<PaginatedResponse<PublicReviewDto>, ContractError> {
        let page = page.max(1);
        let page_size = page_size.clamp(1, 100);
        let offset = (page - 1) * page_size;
        let product_id_str = product_id.to_string();

        let count_row = sqlx::query(
            "SELECT COUNT(*) as cnt FROM product_reviews WHERE product_id = $1 AND is_visible = 1",
        )
        .bind(&product_id_str)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let total: i64 = count_row.get("cnt");
        let total_pages = if total == 0 {
            0
        } else {
            (total + page_size - 1) / page_size
        };

        let rows = sqlx::query(
            r#"
            SELECT r.id, r.product_id, r.buyer_id, COALESCE(b.full_name, 'Pembeli') as buyer_name,
                   r.order_id, r.rating, r.review_text, r.is_visible, r.created_at, r.updated_at
            FROM product_reviews r
            LEFT JOIN buyer_accounts b ON r.buyer_id = b.id
            WHERE r.product_id = $1 AND r.is_visible = 1
            ORDER BY r.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(&product_id_str)
        .bind(page_size)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let mut data = Vec::with_capacity(rows.len());
        for row in &rows {
            data.push(Self::row_to_public_dto(row)?);
        }

        Ok(PaginatedResponse {
            data,
            total,
            page,
            page_size,
            total_pages,
        })
    }

    async fn get_rating_summary(
        &self,
        product_id: Uuid,
    ) -> Result<ProductRatingSummaryDto, ContractError> {
        let product_id_str = product_id.to_string();

        let rows = sqlx::query(
            r#"
            SELECT rating, COUNT(*) as cnt
            FROM product_reviews
            WHERE product_id = $1 AND is_visible = 1
            GROUP BY rating
            "#,
        )
        .bind(&product_id_str)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let mut distribution = [0u64; 5];
        let mut total_reviews = 0u64;
        let mut total_score = 0.0f64;

        for row in rows {
            let r: i64 = row.get("rating");
            let c: i64 = row.get("cnt");
            if (1..=5).contains(&r) {
                let idx = (r - 1) as usize;
                let count = c as u64;
                distribution[idx] = count;
                total_reviews += count;
                total_score += (r as f64) * (count as f64);
            }
        }

        let average_rating = if total_reviews == 0 {
            0.0
        } else {
            let raw_avg = total_score / (total_reviews as f64);
            (raw_avg * 10.0).round() / 10.0
        };

        Ok(ProductRatingSummaryDto {
            product_id,
            average_rating,
            total_reviews,
            rating_distribution: distribution,
        })
    }

    async fn list_buyer_reviews(
        &self,
        buyer_id: Uuid,
    ) -> Result<Vec<ProductReviewDto>, ContractError> {
        let buyer_id_str = buyer_id.to_string();

        let rows = sqlx::query(
            r#"
            SELECT r.id, r.product_id, r.buyer_id, COALESCE(b.full_name, 'Pembeli') as buyer_name,
                   r.order_id, r.rating, r.review_text, r.is_visible, r.created_at, r.updated_at
            FROM product_reviews r
            LEFT JOIN buyer_accounts b ON r.buyer_id = b.id
            WHERE r.buyer_id = $1
            ORDER BY r.created_at DESC
            "#,
        )
        .bind(&buyer_id_str)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let mut data = Vec::with_capacity(rows.len());
        for row in &rows {
            data.push(Self::row_to_dto(row)?);
        }

        Ok(data)
    }

    async fn admin_list_reviews(
        &self,
        product_id: Option<Uuid>,
        is_visible: Option<bool>,
        page: i64,
        page_size: i64,
    ) -> Result<PaginatedResponse<ProductReviewDto>, ContractError> {
        let page = page.max(1);
        let page_size = page_size.clamp(1, 100);
        let offset = (page - 1) * page_size;

        let prod_str = product_id.map(|id| id.to_string());
        let vis_int = is_visible.map(|v| if v { 1i64 } else { 0i64 });

        // Total count
        let count_row = sqlx::query(
            r#"
            SELECT COUNT(*) as cnt
            FROM product_reviews
            WHERE ($1 IS NULL OR product_id = $1)
              AND ($2 IS NULL OR is_visible = $2)
            "#,
        )
        .bind(&prod_str)
        .bind(vis_int)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let total: i64 = count_row.get("cnt");
        let total_pages = if total == 0 {
            0
        } else {
            (total + page_size - 1) / page_size
        };

        let rows = sqlx::query(
            r#"
            SELECT r.id, r.product_id, r.buyer_id, COALESCE(b.full_name, 'Pembeli') as buyer_name,
                   r.order_id, r.rating, r.review_text, r.is_visible, r.created_at, r.updated_at
            FROM product_reviews r
            LEFT JOIN buyer_accounts b ON r.buyer_id = b.id
            WHERE ($1 IS NULL OR r.product_id = $1)
              AND ($2 IS NULL OR r.is_visible = $2)
            ORDER BY r.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(&prod_str)
        .bind(vis_int)
        .bind(page_size)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let mut data = Vec::with_capacity(rows.len());
        for row in &rows {
            data.push(Self::row_to_dto(row)?);
        }

        Ok(PaginatedResponse {
            data,
            total,
            page,
            page_size,
            total_pages,
        })
    }

    async fn admin_update_visibility(
        &self,
        review_id: Uuid,
        is_visible: bool,
        actor_id: Option<Uuid>,
        actor_username: Option<String>,
    ) -> Result<ProductReviewDto, ContractError> {
        let review_id_str = review_id.to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let vis_int = if is_visible { 1i64 } else { 0i64 };

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        // 1. Fetch current review state
        let existing =
            sqlx::query("SELECT product_id, is_visible FROM product_reviews WHERE id = $1")
                .bind(&review_id_str)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| ContractError::Internal(e.to_string()))?;

        let (product_id_str, prev_vis_int): (String, i64) = match existing {
            Some(row) => (row.get("product_id"), row.get("is_visible")),
            None => {
                return Err(ContractError::NotFound(format!(
                    "Review not found: {}",
                    review_id
                )));
            }
        };
        let prev_is_visible = prev_vis_int != 0;

        // 2. Update review visibility
        sqlx::query("UPDATE product_reviews SET is_visible = $1, updated_at = $2 WHERE id = $3")
            .bind(vis_int)
            .bind(&now_str)
            .bind(&review_id_str)
            .execute(&mut *tx)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        // 3. Atomically record immutable audit log entry (fail closed on failure)
        let audit_id = Uuid::new_v4().to_string();
        let actor_id_str = actor_id.map(|id| id.to_string());
        let username = actor_username.unwrap_or_else(|| "admin".to_string());
        let action = if is_visible {
            "review.unhide"
        } else {
            "review.hide"
        };
        let details = serde_json::json!({
            "product_id": product_id_str,
            "previous_is_visible": prev_is_visible,
            "new_is_visible": is_visible,
            "actor_id": actor_id_str,
            "actor_username": username
        })
        .to_string();

        sqlx::query(
            "INSERT INTO audit_logs (id, timestamp, actor_id, actor_username, action, resource_type, resource_id, details, ip_address)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(&audit_id)
        .bind(&now_str)
        .bind(&actor_id_str)
        .bind(&username)
        .bind(action)
        .bind("review")
        .bind(&review_id_str)
        .bind(&details)
        .bind(None::<String>)
        .execute(&mut *tx)
        .await
        .map_err(|e| ContractError::Internal(format!("Audit logging failed: {}", e)))?;

        // 4. Fetch updated review row within transaction
        let row = sqlx::query(
            r#"
            SELECT r.id, r.product_id, r.buyer_id, COALESCE(b.full_name, 'Pembeli') as buyer_name,
                   r.order_id, r.rating, r.review_text, r.is_visible, r.created_at, r.updated_at
            FROM product_reviews r
            LEFT JOIN buyer_accounts b ON r.buyer_id = b.id
            WHERE r.id = $1
            "#,
        )
        .bind(&review_id_str)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        // 5. Commit transaction atomically
        tx.commit()
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        Self::row_to_dto(&row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use program1_core::init_database;

    async fn setup_test_db() -> DbPool {
        init_database("sqlite::memory:")
            .await
            .expect("Failed to initialize in-memory SQLite database")
    }

    async fn insert_test_fixture(
        pool: &DbPool,
        buyer_id: Uuid,
        product_id: Uuid,
        order_id: Uuid,
        order_status: &str,
    ) {
        let buyer_id_str = buyer_id.to_string();
        let prod_id_str = product_id.to_string();
        let ord_id_str = order_id.to_string();

        // 1. Insert buyer
        sqlx::query(
            r#"
            INSERT INTO buyer_accounts (id, google_sub, email, full_name, phone_number, phone_verified, is_active)
            VALUES ($1, $2, $3, $4, $5, 1, 1)
            "#,
        )
        .bind(&buyer_id_str)
        .bind(format!("sub_{}", buyer_id))
        .bind(format!("buyer_{}@example.com", buyer_id))
        .bind("John Doe")
        .bind("08123456789")
        .execute(pool)
        .await
        .expect("Failed to insert buyer");

        // 2. Insert product in catalog_items
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO catalog_items (id, name, sku, category, price, stock, description)
            VALUES ($1, $2, $3, 'Electronics', 100000.0, 50, 'Awesome gadget')
            "#,
        )
        .bind(&prod_id_str)
        .bind("Smart Gadget")
        .bind(format!("SKU-{}", &prod_id_str[0..8]))
        .execute(pool)
        .await
        .expect("Failed to insert product");

        // 3. Insert order
        sqlx::query(
            r#"
            INSERT INTO orders (id, customer_name, customer_email, shipping_address, total_amount, status, buyer_id)
            VALUES ($1, 'John Doe', 'buyer@example.com', 'Jakarta', 100000.0, $2, $3)
            "#,
        )
        .bind(&ord_id_str)
        .bind(order_status)
        .bind(&buyer_id_str)
        .execute(pool)
        .await
        .expect("Failed to insert order");

        // 4. Insert order item
        let item_id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO order_items (id, order_id, product_id, product_name, quantity, unit_price, total_price)
            VALUES ($1, $2, $3, 'Smart Gadget', 1, 100000.0, 100000.0)
            "#,
        )
        .bind(&item_id)
        .bind(&ord_id_str)
        .bind(&prod_id_str)
        .execute(pool)
        .await
        .expect("Failed to insert order item");
    }

    #[tokio::test]
    async fn test_create_review_success_delivered_order() {
        let pool = setup_test_db().await;
        let module = ReviewModule::new(pool.clone());

        let buyer_id = Uuid::new_v4();
        let product_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        insert_test_fixture(&pool, buyer_id, product_id, order_id, "delivered").await;

        let req = CreateReviewRequest {
            product_id,
            order_id,
            rating: 5,
            review_text: Some(
                "Barang sangat bagus dan berkualitas! <script>alert(1)</script>".to_string(),
            ),
        };

        let review = module
            .create_review(buyer_id, req)
            .await
            .expect("Review should succeed");
        assert_eq!(review.rating, 5);
        assert_eq!(review.buyer_id, buyer_id);
        assert_eq!(review.buyer_name, "John Doe");
        assert!(review.is_visible);
        assert_eq!(
            review.review_text.as_deref(),
            Some("Barang sangat bagus dan berkualitas! alert(1)")
        );
    }

    #[tokio::test]
    async fn test_create_review_requires_delivered_order() {
        let pool = setup_test_db().await;
        let module = ReviewModule::new(pool.clone());

        let buyer_id = Uuid::new_v4();
        let product_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        insert_test_fixture(&pool, buyer_id, product_id, order_id, "pending").await;

        let req = CreateReviewRequest {
            product_id,
            order_id,
            rating: 4,
            review_text: Some("Belum sampai tapi mau review".to_string()),
        };

        let result = module.create_review(buyer_id, req).await;
        assert!(matches!(result, Err(ContractError::ValidationError(_))));
    }

    #[tokio::test]
    async fn test_create_review_fails_when_order_not_owned_by_buyer() {
        let pool = setup_test_db().await;
        let module = ReviewModule::new(pool.clone());

        let buyer_id = Uuid::new_v4();
        let product_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();
        let other_buyer_id = Uuid::new_v4();

        insert_test_fixture(&pool, buyer_id, product_id, order_id, "delivered").await;

        let req = CreateReviewRequest {
            product_id,
            order_id,
            rating: 5,
            review_text: Some("Trying to review another buyer's order".to_string()),
        };

        let result = module.create_review(other_buyer_id, req).await;
        assert!(matches!(result, Err(ContractError::ValidationError(_))));
    }

    #[tokio::test]
    async fn test_create_review_fails_when_product_absent_from_order() {
        let pool = setup_test_db().await;
        let module = ReviewModule::new(pool.clone());

        let buyer_id = Uuid::new_v4();
        let product_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();
        let unpurchased_product_id = Uuid::new_v4();

        insert_test_fixture(&pool, buyer_id, product_id, order_id, "delivered").await;

        let req = CreateReviewRequest {
            product_id: unpurchased_product_id,
            order_id,
            rating: 5,
            review_text: Some("Reviewing product not in order".to_string()),
        };

        let result = module.create_review(buyer_id, req).await;
        assert!(matches!(result, Err(ContractError::ValidationError(_))));
    }

    #[tokio::test]
    async fn test_duplicate_review_rejected() {
        let pool = setup_test_db().await;
        let module = ReviewModule::new(pool.clone());

        let buyer_id = Uuid::new_v4();
        let product_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        insert_test_fixture(&pool, buyer_id, product_id, order_id, "delivered").await;

        let req1 = CreateReviewRequest {
            product_id,
            order_id,
            rating: 5,
            review_text: Some("First review".to_string()),
        };
        let _ = module
            .create_review(buyer_id, req1)
            .await
            .expect("First review succeeds");

        let req2 = CreateReviewRequest {
            product_id,
            order_id,
            rating: 4,
            review_text: Some("Second review should fail".to_string()),
        };
        let result = module.create_review(buyer_id, req2).await;
        assert!(matches!(result, Err(ContractError::ValidationError(_))));
    }

    #[tokio::test]
    async fn test_rating_summary_calculation_and_hidden_leakage_prevented() {
        let pool = setup_test_db().await;
        let module = ReviewModule::new(pool.clone());

        let product_id = Uuid::new_v4();

        // Create 3 orders with 3 different buyers and ratings: 3, 4, 5
        let ratings = [3, 4, 5];
        for r in ratings {
            let b_id = Uuid::new_v4();
            let o_id = Uuid::new_v4();
            insert_test_fixture(&pool, b_id, product_id, o_id, "delivered").await;

            let req = CreateReviewRequest {
                product_id,
                order_id: o_id,
                rating: r,
                review_text: Some(format!("Review with rating {}", r)),
            };
            module
                .create_review(b_id, req)
                .await
                .expect("Review created");
        }

        // Check summary before moderation
        let summary = module
            .get_rating_summary(product_id)
            .await
            .expect("Summary ok");
        assert_eq!(summary.total_reviews, 3);
        assert_eq!(summary.average_rating, 4.0); // (3+4+5)/3 = 4.0
        assert_eq!(summary.rating_distribution, [0, 0, 1, 1, 1]); // 3-star:1, 4-star:1, 5-star:1

        // Check public reviews list
        let pub_reviews = module
            .get_reviews_for_product(product_id, 1, 10)
            .await
            .expect("List ok");
        assert_eq!(pub_reviews.total, 3);
        assert_eq!(pub_reviews.data.len(), 3);

        // Moderate: hide the 3-star review
        let rev_3 = pub_reviews.data.iter().find(|r| r.rating == 3).unwrap();
        let admin_id = Uuid::new_v4();
        let updated = module
            .admin_update_visibility(rev_3.id, false, Some(admin_id), Some("admin".to_string()))
            .await
            .expect("Hide ok");
        assert!(!updated.is_visible);

        // Now verify hidden-review leakage is prevented:
        // 1. Not in public reviews
        let pub_after_hide = module
            .get_reviews_for_product(product_id, 1, 10)
            .await
            .expect("List ok");
        assert_eq!(pub_after_hide.total, 2);
        assert_eq!(pub_after_hide.data.len(), 2);
        assert!(!pub_after_hide.data.iter().any(|r| r.id == rev_3.id));

        // 2. Not in rating summary
        let summary_after = module
            .get_rating_summary(product_id)
            .await
            .expect("Summary ok");
        assert_eq!(summary_after.total_reviews, 2);
        assert_eq!(summary_after.average_rating, 4.5); // (4+5)/2 = 4.5
        assert_eq!(summary_after.rating_distribution, [0, 0, 0, 1, 1]);

        // 3. But visible to admin
        let admin_list = module
            .admin_list_reviews(Some(product_id), None, 1, 10)
            .await
            .expect("Admin list ok");
        assert_eq!(admin_list.total, 3);
    }

    #[tokio::test]
    async fn test_invalid_rating_rejected() {
        let pool = setup_test_db().await;
        let module = ReviewModule::new(pool.clone());

        let buyer_id = Uuid::new_v4();
        let product_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        insert_test_fixture(&pool, buyer_id, product_id, order_id, "delivered").await;

        let req_zero = CreateReviewRequest {
            product_id,
            order_id,
            rating: 0,
            review_text: Some("Zero stars".to_string()),
        };
        let res_zero = module.create_review(buyer_id, req_zero).await;
        assert!(matches!(res_zero, Err(ContractError::ValidationError(_))));

        let req_six = CreateReviewRequest {
            product_id,
            order_id,
            rating: 6,
            review_text: Some("Six stars".to_string()),
        };
        let res_six = module.create_review(buyer_id, req_six).await;
        assert!(matches!(res_six, Err(ContractError::ValidationError(_))));
    }

    #[tokio::test]
    async fn test_sanitized_text_and_no_pii_leakage() {
        let pool = setup_test_db().await;
        let module = ReviewModule::new(pool.clone());

        let buyer_id = Uuid::new_v4();
        let product_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        insert_test_fixture(&pool, buyer_id, product_id, order_id, "delivered").await;

        let xss_text = "<b>Bagus sekali!</b> <script>document.cookie='leak'</script> <img src=x onerror=alert(1) /> Recomended!";
        let req = CreateReviewRequest {
            product_id,
            order_id,
            rating: 5,
            review_text: Some(xss_text.to_string()),
        };

        let review = module
            .create_review(buyer_id, req)
            .await
            .expect("Create ok");
        assert_eq!(review.rating, 5);
        assert!(!review.review_text.as_ref().unwrap().contains("<script>"));
        assert!(!review.review_text.as_ref().unwrap().contains("<b>"));
        assert_eq!(review.buyer_name, "John Doe");

        // Serialize to JSON and verify buyer_id / buyer_name are present, but email/phone are NOT present
        let json_val = serde_json::to_value(&review).unwrap();
        assert!(json_val.get("buyer_name").is_some());
        assert!(json_val.get("email").is_none());
        assert!(json_val.get("phone").is_none());
        assert!(json_val.get("phone_number").is_none());
    }

    #[tokio::test]
    async fn test_list_buyer_reviews() {
        let pool = setup_test_db().await;
        let module = ReviewModule::new(pool.clone());

        let buyer_id = Uuid::new_v4();
        let product_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        insert_test_fixture(&pool, buyer_id, product_id, order_id, "delivered").await;

        let req = CreateReviewRequest {
            product_id,
            order_id,
            rating: 4,
            review_text: Some("Mantap".to_string()),
        };
        module
            .create_review(buyer_id, req)
            .await
            .expect("Review created");

        let buyer_reviews = module
            .list_buyer_reviews(buyer_id)
            .await
            .expect("List buyer reviews ok");
        assert_eq!(buyer_reviews.len(), 1);
        assert_eq!(buyer_reviews[0].product_id, product_id);

        let other_buyer_reviews = module
            .list_buyer_reviews(Uuid::new_v4())
            .await
            .expect("Empty for other buyer");
        assert_eq!(other_buyer_reviews.len(), 0);
    }

    #[tokio::test]
    async fn test_admin_moderation_atomic_audit_failure_rollback() {
        let pool = setup_test_db().await;
        let module = ReviewModule::new(pool.clone());

        let buyer_id = Uuid::new_v4();
        let product_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        insert_test_fixture(&pool, buyer_id, product_id, order_id, "delivered").await;

        let req = CreateReviewRequest {
            product_id,
            order_id,
            rating: 5,
            review_text: Some("Excellent product".to_string()),
        };
        let review = module
            .create_review(buyer_id, req)
            .await
            .expect("Create review ok");
        assert!(review.is_visible);

        // Install a trigger that aborts any INSERT into audit_logs to simulate audit failure
        sqlx::query(
            "CREATE TRIGGER fail_audit_insert BEFORE INSERT ON audit_logs BEGIN SELECT RAISE(ABORT, 'Simulated audit log insertion failure'); END;"
        )
        .execute(&pool)
        .await
        .expect("Trigger created");

        let admin_id = Uuid::new_v4();
        // Attempt moderation - should fail closed because audit insert fails
        let res = module
            .admin_update_visibility(
                review.id,
                false,
                Some(admin_id),
                Some("admin_moderator".to_string()),
            )
            .await;

        assert!(
            res.is_err(),
            "Expected moderation to fail closed when audit logging fails"
        );

        // Verify transaction rolled back: review is STILL visible!
        let row = sqlx::query("SELECT is_visible FROM product_reviews WHERE id = $1")
            .bind(review.id.to_string())
            .fetch_one(&pool)
            .await
            .expect("Fetch review row");
        let is_visible_int: i64 = row.get("is_visible");
        assert_eq!(
            is_visible_int, 1,
            "Review visibility must roll back to 1 (visible) if audit logging fails"
        );

        // Verify no orphaned audit log entry
        let count_row =
            sqlx::query("SELECT COUNT(*) as cnt FROM audit_logs WHERE resource_id = $1")
                .bind(review.id.to_string())
                .fetch_one(&pool)
                .await
                .expect("Fetch audit count");
        let audit_count: i64 = count_row.get("cnt");
        assert_eq!(audit_count, 0, "No audit logs should exist after rollback");

        // Now drop trigger and verify successful atomic mutation and audit recording
        sqlx::query("DROP TRIGGER fail_audit_insert")
            .execute(&pool)
            .await
            .expect("Drop trigger");

        let success_res = module
            .admin_update_visibility(
                review.id,
                false,
                Some(admin_id),
                Some("admin_moderator".to_string()),
            )
            .await
            .expect("Moderation succeeds after trigger dropped");
        assert!(!success_res.is_visible);

        // Verify audit log entry was created atomically with details
        let audit_row = sqlx::query(
            "SELECT action, actor_id, actor_username, details FROM audit_logs WHERE resource_id = $1",
        )
        .bind(review.id.to_string())
        .fetch_one(&pool)
        .await
        .expect("Fetch audit log");

        let action: String = audit_row.get("action");
        let actor_username: String = audit_row.get("actor_username");
        let details: String = audit_row.get("details");

        assert_eq!(action, "review.hide");
        assert_eq!(actor_username, "admin_moderator");
        assert!(details.contains(&product_id.to_string()));
        assert!(details.contains("\"previous_is_visible\":true"));
        assert!(details.contains("\"new_is_visible\":false"));
        assert!(details.contains(&admin_id.to_string()));
    }
}
