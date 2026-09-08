use async_trait::async_trait;
use chrono::{DateTime, Utc};
use program1_contracts::{
    CatalogContract, CatalogItemDto, CategoryDto, ContractError, CreateCatalogItemRequest,
    CreateCategoryRequest, CreateVariantRequest, PaginatedResponse, ProductVariantDto,
    UpdateCategoryRequest, UpdateVariantRequest,
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
        Self { pool }
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

        let now = Utc::now().to_rfc3339();
        for (id, name, sku, category, price, stock, img, desc) in initial_items {
            let cat_id = match category {
                "Peripherals" => Some("cat-002"),
                "Accessories" => Some("cat-003"),
                _ => None,
            };
            sqlx::query(
                "INSERT OR IGNORE INTO catalog_items (id, name, sku, category, price, stock, image_url, description, category_id, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            )
            .bind(id)
            .bind(name)
            .bind(sku)
            .bind(category)
            .bind(price)
            .bind(stock)
            .bind(img)
            .bind(desc)
            .bind(cat_id)
            .bind(&now)
            .execute(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;
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
        let weight_grams: i64 = row.try_get("weight_grams").unwrap_or(500);
        let category_id: Option<String> = row.try_get("category_id").ok();
        let category_name: Option<String> = row.try_get("category_name").ok();

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
            weight_grams,
            category_id,
            category_name,
        })
    }

    fn category_row_to_dto(row: &sqlx::sqlite::SqliteRow) -> Result<CategoryDto, ContractError> {
        let id: String = row.get("id");
        let name: String = row.get("name");
        let slug: String = row.get("slug");
        let description: Option<String> = row.try_get("description").ok();
        let icon: Option<String> = row.try_get("icon").ok();
        let sort_order: i64 = row.try_get("sort_order").unwrap_or(0);
        let is_active_int: i64 = row.try_get("is_active").unwrap_or(1);
        let product_count: i64 = row.try_get("product_count").unwrap_or(0);

        let created_at_str: String = row.get("created_at");
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(CategoryDto {
            id,
            name,
            slug,
            description,
            icon,
            sort_order: sort_order as i32,
            is_active: is_active_int != 0,
            product_count,
            created_at,
        })
    }

    fn variant_row_to_dto(
        row: &sqlx::sqlite::SqliteRow,
    ) -> Result<ProductVariantDto, ContractError> {
        let id_str: String = row.get("id");
        let id = Uuid::parse_str(&id_str)
            .map_err(|e| ContractError::Internal(format!("Corrupt UUID in variant id: {}", e)))?;

        let product_id_str: String = row.get("product_id");
        let product_id = Uuid::parse_str(&product_id_str).map_err(|e| {
            ContractError::Internal(format!("Corrupt UUID in variant product_id: {}", e))
        })?;

        let variant_name: String = row.get("variant_name");
        let variant_value: String = row.get("variant_value");
        let sku: Option<String> = row.get("sku");
        let price_override: Option<f64> = row.get("price_override");
        let stock_quantity: i64 = row.get("stock_quantity");
        let is_active_int: i64 = row.get("is_active");
        let is_active = is_active_int != 0;

        let created_at_str: String = row.get("created_at");
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let updated_at_str: String = row.get("updated_at");
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(ProductVariantDto {
            id,
            product_id,
            variant_name,
            variant_value,
            sku,
            price_override,
            stock_quantity: stock_quantity as u32,
            is_active,
            created_at,
            updated_at,
        })
    }
}

#[async_trait]
impl CatalogContract for CatalogModule {
    async fn list_items(&self) -> Result<Vec<CatalogItemDto>, ContractError> {
        let rows = sqlx::query(
            "SELECT c.id, c.name, c.sku, c.category, c.price, c.stock, c.image_url, c.description, c.created_at, c.weight_grams, c.category_id, cat.name as category_name
             FROM catalog_items c
             LEFT JOIN categories cat ON c.category_id = cat.id
             ORDER BY c.created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        rows.iter().map(Self::row_to_dto).collect()
    }

    async fn list_items_paginated(
        &self,
        page: i64,
        page_size: i64,
        search: Option<&str>,
        category: Option<&str>,
        sort_by: Option<&str>,
        sort_order: Option<&str>,
    ) -> Result<PaginatedResponse<CatalogItemDto>, ContractError> {
        let page = page.max(1);
        let page_size = page_size.clamp(1, 100);
        let offset = (page - 1) * page_size;

        let search_term = search.map(|s| s.trim()).filter(|s| !s.is_empty());
        let category_term = category.map(|c| c.trim()).filter(|c| !c.is_empty() && *c != "all" && *c != "semua");

        let order_col = match sort_by.unwrap_or("created_at").to_lowercase().as_str() {
            "name" => "c.name",
            "price" => "c.price",
            "stock" => "c.stock",
            "sku" => "c.sku",
            _ => "c.created_at",
        };
        let order_dir = match sort_order.unwrap_or("desc").to_lowercase().as_str() {
            "asc" => "ASC",
            _ => "DESC",
        };

        let count_row = sqlx::query(
            "SELECT COUNT(*) as total FROM catalog_items c
             LEFT JOIN categories cat ON c.category_id = cat.id
             WHERE ($1 IS NULL OR c.name LIKE '%' || $1 || '%' OR c.sku LIKE '%' || $1 || '%')
               AND ($2 IS NULL OR LOWER(c.category) = LOWER($2) OR c.category_id = $2 OR LOWER(cat.slug) = LOWER($2) OR LOWER(cat.name) = LOWER($2))",
        )
        .bind(search_term)
        .bind(category_term)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let total: i64 = count_row.get("total");

        let query_str = format!(
            "SELECT c.id, c.name, c.sku, c.category, c.price, c.stock, c.image_url, c.description, c.created_at, c.weight_grams, c.category_id, cat.name as category_name
             FROM catalog_items c
             LEFT JOIN categories cat ON c.category_id = cat.id
             WHERE ($1 IS NULL OR c.name LIKE '%' || $1 || '%' OR c.sku LIKE '%' || $1 || '%')
               AND ($2 IS NULL OR LOWER(c.category) = LOWER($2) OR c.category_id = $2 OR LOWER(cat.slug) = LOWER($2) OR LOWER(cat.name) = LOWER($2))
             ORDER BY {} {}
             LIMIT $3 OFFSET $4",
            order_col, order_dir
        );

        let rows = sqlx::query(&query_str)
            .bind(search_term)
            .bind(category_term)
            .bind(page_size)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let data: Result<Vec<CatalogItemDto>, ContractError> =
            rows.iter().map(Self::row_to_dto).collect();
        let data = data?;

        let total_pages = if total == 0 {
            0
        } else {
            (total + page_size - 1) / page_size
        };

        Ok(PaginatedResponse {
            data,
            total,
            page,
            page_size,
            total_pages,
        })
    }

    async fn get_item(&self, id: Uuid) -> Result<CatalogItemDto, ContractError> {
        let row = sqlx::query(
            "SELECT c.id, c.name, c.sku, c.category, c.price, c.stock, c.image_url, c.description, c.created_at, c.weight_grams, c.category_id, cat.name as category_name
             FROM catalog_items c
             LEFT JOIN categories cat ON c.category_id = cat.id
             WHERE c.id = $1",
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

    async fn create_item(
        &self,
        req: CreateCatalogItemRequest,
    ) -> Result<CatalogItemDto, ContractError> {
        if req.name.trim().is_empty() {
            return Err(ContractError::ValidationError(
                "Item name cannot be empty".to_string(),
            ));
        }
        if req.price < 0.0 {
            return Err(ContractError::ValidationError(
                "Price cannot be negative".to_string(),
            ));
        }

        let id = Uuid::new_v4();
        let sku = req.sku.trim().to_uppercase();
        let name = req.name.trim().to_string();
        let mut category = req.category.trim().to_string();
        let cat_id = req.category_id.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());
        let mut cat_name = None;

        if let Some(cid) = cat_id {
            if let Ok(Some(row)) = sqlx::query("SELECT name FROM categories WHERE id = $1")
                .bind(cid)
                .fetch_optional(&self.pool)
                .await
            {
                let db_cat_name: String = row.get("name");
                if category.is_empty() {
                    category = db_cat_name.clone();
                }
                cat_name = Some(db_cat_name);
            }
        }

        let image_url = req
            .image_url
            .unwrap_or_else(|| "https://via.placeholder.com/500".to_string());
        let description = req
            .description
            .unwrap_or_else(|| "Product description".to_string());
        let now = Utc::now();

        let weight = if req.weight_grams > 0 {
            req.weight_grams
        } else {
            500
        };

        sqlx::query(
            "INSERT INTO catalog_items (id, name, sku, category, price, stock, image_url, description, created_at, weight_grams, category_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
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
        .bind(weight)
        .bind(cat_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        tracing::info!(id = %id, sku = %sku, weight = weight, category = %category, "Catalog item created in database");

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
            weight_grams: weight,
            category_id: cat_id.map(|s| s.to_string()),
            category_name: cat_name,
        })
    }

    async fn list_variants(
        &self,
        product_id: Uuid,
    ) -> Result<Vec<ProductVariantDto>, ContractError> {
        let rows = sqlx::query(
            "SELECT id, product_id, variant_name, variant_value, sku, price_override, stock_quantity, is_active, created_at, updated_at
             FROM product_variants
             WHERE product_id = $1 AND is_active = 1
             ORDER BY variant_name ASC, variant_value ASC",
        )
        .bind(product_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        rows.iter().map(Self::variant_row_to_dto).collect()
    }

    async fn get_variant(&self, variant_id: Uuid) -> Result<ProductVariantDto, ContractError> {
        let row = sqlx::query(
            "SELECT id, product_id, variant_name, variant_value, sku, price_override, stock_quantity, is_active, created_at, updated_at
             FROM product_variants
             WHERE id = $1",
        )
        .bind(variant_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        match row {
            Some(r) => Self::variant_row_to_dto(&r),
            None => Err(ContractError::NotFound(format!(
                "Product Variant {}",
                variant_id
            ))),
        }
    }

    async fn create_variant(
        &self,
        product_id: Uuid,
        req: CreateVariantRequest,
    ) -> Result<ProductVariantDto, ContractError> {
        let variant_name = req.variant_name.trim().to_string();
        let variant_value = req.variant_value.trim().to_string();

        if variant_name.is_empty() {
            return Err(ContractError::ValidationError(
                "Variant name cannot be empty".to_string(),
            ));
        }
        if variant_value.is_empty() {
            return Err(ContractError::ValidationError(
                "Variant value cannot be empty".to_string(),
            ));
        }
        if let Some(price) = req.price_override {
            if price < 0.0 {
                return Err(ContractError::ValidationError(
                    "Price override cannot be negative".to_string(),
                ));
            }
        }

        // Check parent item exists
        let prod = sqlx::query("SELECT id FROM catalog_items WHERE id = $1")
            .bind(product_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        if prod.is_none() {
            return Err(ContractError::NotFound(format!(
                "Catalog item {}",
                product_id
            )));
        }

        // Check duplicate
        let duplicate = sqlx::query(
            "SELECT id FROM product_variants WHERE product_id = $1 AND LOWER(variant_name) = LOWER($2) AND LOWER(variant_value) = LOWER($3)",
        )
        .bind(product_id.to_string())
        .bind(&variant_name)
        .bind(&variant_value)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        if duplicate.is_some() {
            return Err(ContractError::ValidationError(format!(
                "Variant with name '{}' and value '{}' already exists for this product",
                variant_name, variant_value
            )));
        }

        let id = Uuid::new_v4();
        let now = Utc::now();
        let sku = req
            .sku
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty());

        sqlx::query(
            "INSERT INTO product_variants (id, product_id, variant_name, variant_value, sku, price_override, stock_quantity, is_active, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, 1, $8, $9)",
        )
        .bind(id.to_string())
        .bind(product_id.to_string())
        .bind(&variant_name)
        .bind(&variant_value)
        .bind(&sku)
        .bind(req.price_override)
        .bind(req.stock_quantity as i64)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        tracing::info!(id = %id, product_id = %product_id, name = %variant_name, value = %variant_value, "Product variant created");

        Ok(ProductVariantDto {
            id,
            product_id,
            variant_name,
            variant_value,
            sku,
            price_override: req.price_override,
            stock_quantity: req.stock_quantity,
            is_active: true,
            created_at: now,
            updated_at: now,
        })
    }

    async fn update_variant(
        &self,
        variant_id: Uuid,
        req: UpdateVariantRequest,
    ) -> Result<ProductVariantDto, ContractError> {
        let existing = self.get_variant(variant_id).await?;

        let variant_name = req.variant_name.trim().to_string();
        let variant_value = req.variant_value.trim().to_string();

        if variant_name.is_empty() {
            return Err(ContractError::ValidationError(
                "Variant name cannot be empty".to_string(),
            ));
        }
        if variant_value.is_empty() {
            return Err(ContractError::ValidationError(
                "Variant value cannot be empty".to_string(),
            ));
        }
        if let Some(price) = req.price_override {
            if price < 0.0 {
                return Err(ContractError::ValidationError(
                    "Price override cannot be negative".to_string(),
                ));
            }
        }

        // Check duplicate if name or value changed
        if variant_name.to_lowercase() != existing.variant_name.to_lowercase()
            || variant_value.to_lowercase() != existing.variant_value.to_lowercase()
        {
            let duplicate = sqlx::query(
                "SELECT id FROM product_variants WHERE product_id = $1 AND LOWER(variant_name) = LOWER($2) AND LOWER(variant_value) = LOWER($3) AND id != $4",
            )
            .bind(existing.product_id.to_string())
            .bind(&variant_name)
            .bind(&variant_value)
            .bind(variant_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

            if duplicate.is_some() {
                return Err(ContractError::ValidationError(format!(
                    "Variant with name '{}' and value '{}' already exists for this product",
                    variant_name, variant_value
                )));
            }
        }

        let now = Utc::now();
        let sku = req
            .sku
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty());
        let is_active = req.is_active.unwrap_or(existing.is_active);

        sqlx::query(
            "UPDATE product_variants
             SET variant_name = $1, variant_value = $2, sku = $3, price_override = $4, stock_quantity = $5, is_active = $6, updated_at = $7
             WHERE id = $8",
        )
        .bind(&variant_name)
        .bind(&variant_value)
        .bind(&sku)
        .bind(req.price_override)
        .bind(req.stock_quantity as i64)
        .bind(if is_active { 1 } else { 0 })
        .bind(now.to_rfc3339())
        .bind(variant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        Ok(ProductVariantDto {
            id: variant_id,
            product_id: existing.product_id,
            variant_name,
            variant_value,
            sku,
            price_override: req.price_override,
            stock_quantity: req.stock_quantity,
            is_active,
            created_at: existing.created_at,
            updated_at: now,
        })
    }

    async fn delete_variant(&self, variant_id: Uuid) -> Result<(), ContractError> {
        let res = sqlx::query("DELETE FROM product_variants WHERE id = $1")
            .bind(variant_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        if res.rows_affected() == 0 {
            return Err(ContractError::NotFound(format!(
                "Product Variant {}",
                variant_id
            )));
        }

        Ok(())
    }

    // --- Category Management ---
    async fn list_categories(&self) -> Result<Vec<CategoryDto>, ContractError> {
        let rows = sqlx::query(
            "SELECT c.id, c.name, c.slug, c.description, c.icon, c.sort_order, c.is_active, c.created_at,
                    (SELECT COUNT(*) FROM catalog_items p WHERE p.category_id = c.id OR p.category = c.name) as product_count
             FROM categories c
             WHERE c.is_active = 1
             ORDER BY c.sort_order ASC, c.name ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        rows.iter().map(Self::category_row_to_dto).collect()
    }

    async fn get_category(&self, id_or_slug: &str) -> Result<CategoryDto, ContractError> {
        let row = sqlx::query(
            "SELECT c.id, c.name, c.slug, c.description, c.icon, c.sort_order, c.is_active, c.created_at,
                    (SELECT COUNT(*) FROM catalog_items p WHERE p.category_id = c.id OR p.category = c.name) as product_count
             FROM categories c
             WHERE c.id = $1 OR c.slug = $1",
        )
        .bind(id_or_slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        match row {
            Some(r) => Self::category_row_to_dto(&r),
            None => Err(ContractError::NotFound(format!("Category {}", id_or_slug))),
        }
    }

    async fn create_category(
        &self,
        req: CreateCategoryRequest,
    ) -> Result<CategoryDto, ContractError> {
        let name = req.name.trim().to_string();
        let slug = req.slug.trim().to_lowercase().replace(' ', "-");
        if name.is_empty() || slug.is_empty() {
            return Err(ContractError::ValidationError("Name and slug cannot be empty".to_string()));
        }

        // Check uniqueness
        let existing = sqlx::query("SELECT id FROM categories WHERE name = $1 OR slug = $2")
            .bind(&name)
            .bind(&slug)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        if existing.is_some() {
            return Err(ContractError::AlreadyExists(
                "Kategori dengan nama atau slug ini sudah ada".to_string(),
            ));
        }

        let id = Uuid::new_v4().to_string();
        let sort_order = req.sort_order.unwrap_or(0) as i64;
        let icon = req.icon.unwrap_or_else(|| "📦".to_string());
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO categories (id, name, slug, description, icon, sort_order, is_active, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, 1, $7)",
        )
        .bind(&id)
        .bind(&name)
        .bind(&slug)
        .bind(&req.description)
        .bind(&icon)
        .bind(sort_order)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        Ok(CategoryDto {
            id,
            name,
            slug,
            description: req.description,
            icon: Some(icon),
            sort_order: sort_order as i32,
            is_active: true,
            product_count: 0,
            created_at: now,
        })
    }

    async fn update_category(
        &self,
        id: &str,
        req: UpdateCategoryRequest,
    ) -> Result<CategoryDto, ContractError> {
        let name = req.name.trim().to_string();
        let slug = req.slug.trim().to_lowercase().replace(' ', "-");
        if name.is_empty() || slug.is_empty() {
            return Err(ContractError::ValidationError("Name and slug cannot be empty".to_string()));
        }

        // Check existence
        let existing = self.get_category(id).await?;

        // Check duplicate name or slug on OTHER category
        let duplicate = sqlx::query("SELECT id FROM categories WHERE (name = $1 OR slug = $2) AND id != $3")
            .bind(&name)
            .bind(&slug)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        if duplicate.is_some() {
            return Err(ContractError::AlreadyExists(
                "Kategori dengan nama atau slug ini sudah digunakan oleh kategori lain".to_string(),
            ));
        }

        let sort_order = req.sort_order.unwrap_or(existing.sort_order) as i64;
        let icon = req.icon.or(existing.icon);
        let description = req.description.or(existing.description);
        let is_active = req.is_active.unwrap_or(existing.is_active);

        sqlx::query(
            "UPDATE categories
             SET name = $1, slug = $2, description = $3, icon = $4, sort_order = $5, is_active = $6
             WHERE id = $7",
        )
        .bind(&name)
        .bind(&slug)
        .bind(&description)
        .bind(&icon)
        .bind(sort_order)
        .bind(if is_active { 1 } else { 0 })
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        Ok(CategoryDto {
            id: id.to_string(),
            name,
            slug,
            description,
            icon,
            sort_order: sort_order as i32,
            is_active,
            product_count: existing.product_count,
            created_at: existing.created_at,
        })
    }

    async fn delete_category(&self, id: &str) -> Result<(), ContractError> {
        let cat = self.get_category(id).await?;
        if cat.product_count > 0 {
            return Err(ContractError::ValidationError(format!(
                "Kategori masih memiliki {} produk terhubung",
                cat.product_count
            )));
        }

        let res = sqlx::query("DELETE FROM categories WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        if res.rows_affected() == 0 {
            return Err(ContractError::NotFound(format!("Category {}", id)));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use program1_core::init_database;

    async fn create_test_catalog_module() -> CatalogModule {
        let pool = init_database("sqlite::memory:")
            .await
            .expect("In-memory SQLite init failed");
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
                category_id: None,
                price: 250000.0,
                stock: 20,
                image_url: None,
                description: None,
                weight_grams: 500,
            })
            .await
            .unwrap();

        assert_eq!(created.name, "Desk Mat XL");
        let fetched = module.get_item(created.id).await.unwrap();
        assert_eq!(fetched.sku, "SKU-MAT-99");
    }

    #[tokio::test]
    async fn test_variant_crud() {
        let module = create_test_catalog_module().await;
        let items = module.list_items().await.unwrap();
        let product_id = items[0].id;

        // 1. Create 3 variants (S, M, L)
        let v1 = module
            .create_variant(
                product_id,
                CreateVariantRequest {
                    variant_name: "Ukuran".to_string(),
                    variant_value: "S".to_string(),
                    sku: Some("SKU-VAR-S".to_string()),
                    price_override: None,
                    stock_quantity: 10,
                },
            )
            .await
            .unwrap();

        let v2 = module
            .create_variant(
                product_id,
                CreateVariantRequest {
                    variant_name: "Ukuran".to_string(),
                    variant_value: "M".to_string(),
                    sku: Some("SKU-VAR-M".to_string()),
                    price_override: Some(1500000.0),
                    stock_quantity: 15,
                },
            )
            .await
            .unwrap();

        let _v3 = module
            .create_variant(
                product_id,
                CreateVariantRequest {
                    variant_name: "Ukuran".to_string(),
                    variant_value: "L".to_string(),
                    sku: Some("SKU-VAR-L".to_string()),
                    price_override: Some(1600000.0),
                    stock_quantity: 20,
                },
            )
            .await
            .unwrap();

        // 2. List variants -> should have 3
        let list = module.list_variants(product_id).await.unwrap();
        assert_eq!(list.len(), 3);

        // 3. Update variant v2
        let updated = module
            .update_variant(
                v2.id,
                UpdateVariantRequest {
                    variant_name: "Ukuran".to_string(),
                    variant_value: "Medium-Updated".to_string(),
                    sku: Some("SKU-VAR-M-UPD".to_string()),
                    price_override: Some(1550000.0),
                    stock_quantity: 25,
                    is_active: Some(true),
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.variant_value, "Medium-Updated");
        assert_eq!(updated.stock_quantity, 25);

        // 4. Delete 1 variant -> should have 2 left
        module.delete_variant(v1.id).await.unwrap();
        let list_after = module.list_variants(product_id).await.unwrap();
        assert_eq!(list_after.len(), 2);
    }

    #[tokio::test]
    async fn test_variant_price_override() {
        let module = create_test_catalog_module().await;
        let items = module.list_items().await.unwrap();
        let product_id = items[0].id;

        let v = module
            .create_variant(
                product_id,
                CreateVariantRequest {
                    variant_name: "Warna".to_string(),
                    variant_value: "Matte Black".to_string(),
                    sku: Some("SKU-VAR-BLK".to_string()),
                    price_override: Some(1750000.0),
                    stock_quantity: 5,
                },
            )
            .await
            .unwrap();

        assert_eq!(v.price_override, Some(1750000.0));
        let fetched = module.get_variant(v.id).await.unwrap();
        assert_eq!(fetched.price_override, Some(1750000.0));
    }

    #[tokio::test]
    async fn test_duplicate_variant_rejected() {
        let module = create_test_catalog_module().await;
        let items = module.list_items().await.unwrap();
        let product_id = items[0].id;

        module
            .create_variant(
                product_id,
                CreateVariantRequest {
                    variant_name: "Ukuran".to_string(),
                    variant_value: "XL".to_string(),
                    sku: None,
                    price_override: None,
                    stock_quantity: 5,
                },
            )
            .await
            .unwrap();

        let dup = module
            .create_variant(
                product_id,
                CreateVariantRequest {
                    variant_name: "Ukuran".to_string(),
                    variant_value: "XL".to_string(),
                    sku: None,
                    price_override: None,
                    stock_quantity: 10,
                },
            )
            .await;

        assert!(dup.is_err());
    }
}
