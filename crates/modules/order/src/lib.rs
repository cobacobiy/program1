use async_trait::async_trait;
use chrono::{DateTime, Utc};
use program1_contracts::{
    CatalogContract, ChannelType, ContractError, InventoryContract, OmniOrderDto, OrderContract,
    OrderItemDto, OrderStatus, PaginatedResponse, ShippingAddressSnapshot,
    StorefrontOrderItemRequest, StorefrontOrderRequest,
};
use program1_core::database::DbPool;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct OrderModule {
    pool: DbPool,
    catalog_contract: Arc<dyn CatalogContract>,
    inventory_contract: Arc<dyn InventoryContract>,
    email_sender: Option<Arc<dyn program1_core::EmailSender>>,
    store_name: String,
}

impl OrderModule {
    pub fn new(
        pool: DbPool,
        catalog_contract: Arc<dyn CatalogContract>,
        inventory_contract: Arc<dyn InventoryContract>,
    ) -> Self {
        Self {
            pool,
            catalog_contract,
            inventory_contract,
            email_sender: None,
            store_name: "AURA Storefront".to_string(),
        }
    }

    pub fn with_email_sender(
        mut self,
        email_sender: Arc<dyn program1_core::EmailSender>,
        store_name: impl Into<String>,
    ) -> Self {
        self.email_sender = Some(email_sender);
        self.store_name = store_name.into();
        self
    }

    fn channel_from_str(s: &str) -> ChannelType {
        match s.to_lowercase().as_str() {
            "tiktok" | "tiktokshop" => ChannelType::TikTokShop,
            "shopee" => ChannelType::Shopee,
            "tokopedia" => ChannelType::Tokopedia,
            _ => ChannelType::NativeWeb,
        }
    }

    async fn fetch_items_for_order(
        &self,
        order_id: &str,
    ) -> Result<Vec<OrderItemDto>, ContractError> {
        let rows = sqlx::query(
            "SELECT product_id, product_name, quantity, unit_price, total_price
             FROM order_items WHERE order_id = $1",
        )
        .bind(order_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let mut items = Vec::new();
        for r in rows {
            let pid_str: String = r.get("product_id");
            let pid = Uuid::parse_str(&pid_str)
                .map_err(|e| ContractError::Internal(format!("Invalid UUID: {}", e)))?;
            let name: String = r.get("product_name");
            let qty: i64 = r.get("quantity");
            let unit_p: f64 = r.get("unit_price");
            let total_p: f64 = r.get("total_price");

            items.push(OrderItemDto {
                product_id: pid,
                product_name: name,
                quantity: qty as u32,
                unit_price: unit_p,
                total_price: total_p,
            });
        }
        Ok(items)
    }

    fn row_to_order_dto(
        &self,
        row: &sqlx::sqlite::SqliteRow,
        items: Vec<OrderItemDto>,
    ) -> Result<OmniOrderDto, ContractError> {
        let id_str: String = row.get("id");
        let id = Uuid::parse_str(&id_str)
            .map_err(|e| ContractError::Internal(format!("Invalid UUID: {}", e)))?;
        let ch_str: String = row.get("channel");
        let customer_name: String = row.get("customer_name");
        let customer_email: String = row.get("customer_email");
        let shipping_address: String = row.get("shipping_address");
        let total_amount: f64 = row.get("total_amount");
        let status: String = row.get("status");
        let created_at_str: String = row.get("created_at");
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let buyer_id_opt: Option<String> = row.try_get("buyer_id").ok().flatten();
        let buyer_id = buyer_id_opt.and_then(|s| Uuid::parse_str(&s).ok());

        let snapshot_json_opt: Option<String> =
            row.try_get("shipping_snapshot_json").ok().flatten();
        let shipping_snapshot: Option<ShippingAddressSnapshot> =
            snapshot_json_opt.and_then(|j| serde_json::from_str(&j).ok());

        let tracking_number: Option<String> = row.try_get("tracking_number").ok().flatten();
        let shipped_at = row
            .try_get::<Option<String>, _>("shipped_at")
            .ok()
            .flatten()
            .and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            });
        let delivered_at = row
            .try_get::<Option<String>, _>("delivered_at")
            .ok()
            .flatten()
            .and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            });
        let cancelled_at = row
            .try_get::<Option<String>, _>("cancelled_at")
            .ok()
            .flatten()
            .and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            });
        let cancelled_by: Option<String> = row.try_get("cancelled_by").ok().flatten();
        let cancel_reason: Option<String> = row.try_get("cancel_reason").ok().flatten();

        Ok(OmniOrderDto {
            id,
            channel: Self::channel_from_str(&ch_str),
            customer_name,
            customer_email,
            shipping_address,
            items,
            total_amount,
            status,
            created_at,
            buyer_id,
            shipping_snapshot,
            tracking_number,
            shipped_at,
            delivered_at,
            cancelled_at,
            cancelled_by,
            cancel_reason,
        })
    }
}

#[async_trait]
impl OrderContract for OrderModule {
    async fn get_order(&self, id: Uuid) -> Result<OmniOrderDto, ContractError> {
        let row = sqlx::query(
            "SELECT id, channel, customer_name, customer_email, shipping_address, total_amount, status, created_at, buyer_id, shipping_snapshot_json, tracking_number, shipped_at, delivered_at, cancelled_at, cancelled_by, cancel_reason
             FROM orders WHERE id = $1",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let row = match row {
            Some(r) => r,
            None => return Err(ContractError::NotFound(format!("Order {}", id))),
        };

        let items = self.fetch_items_for_order(&id.to_string()).await?;
        self.row_to_order_dto(&row, items)
    }

    async fn create_storefront_order(
        &self,
        req: StorefrontOrderRequest,
    ) -> Result<OmniOrderDto, ContractError> {
        if req.items.is_empty() {
            return Err(ContractError::ValidationError(
                "Order must contain at least one item".to_string(),
            ));
        }
        if req.customer_name.trim().is_empty() {
            return Err(ContractError::ValidationError(
                "Customer name is required".to_string(),
            ));
        }

        let mut order_items = Vec::new();
        let mut total_amount = 0.0;

        for item in &req.items {
            if item.quantity == 0 {
                return Err(ContractError::ValidationError(
                    "Item quantity must be > 0".to_string(),
                ));
            }

            let product = self.catalog_contract.get_item(item.product_id).await?;
            self.inventory_contract
                .reserve_stock(item.product_id, item.quantity)
                .await?;

            let item_total = product.price * (item.quantity as f64);
            total_amount += item_total;

            order_items.push(OrderItemDto {
                product_id: product.id,
                product_name: product.name,
                quantity: item.quantity,
                unit_price: product.price,
                total_price: item_total,
            });
        }

        let order_id = Uuid::new_v4();
        let now = Utc::now();
        let channel_str = "NativeWeb";
        let status_str = "pending";

        let snapshot = match req.shipping_snapshot {
            Some(s) => s,
            None => ShippingAddressSnapshot {
                recipient_name: req.customer_name.trim().to_string(),
                phone_number: "".to_string(),
                street_address: req.shipping_address.trim().to_string(),
                subdistrict: "".to_string(),
                city: "".to_string(),
                province: "".to_string(),
                postal_code: "".to_string(),
            },
        };
        let snapshot_json = serde_json::to_string(&snapshot).unwrap_or_default();
        let buyer_id_str = req.buyer_id.map(|b| b.to_string());

        // Insert order with immutable shipping snapshot
        sqlx::query(
            "INSERT INTO orders (id, channel, customer_name, customer_email, shipping_address, total_amount, status, created_at, buyer_id, shipping_recipient_name, shipping_phone_number, shipping_street_address, shipping_subdistrict, shipping_city, shipping_province, shipping_postal_code, shipping_snapshot_json)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)",
        )
        .bind(order_id.to_string())
        .bind(channel_str)
        .bind(req.customer_name.trim())
        .bind(req.customer_email.trim())
        .bind(req.shipping_address.trim())
        .bind(total_amount)
        .bind(status_str)
        .bind(now.to_rfc3339())
        .bind(&buyer_id_str)
        .bind(&snapshot.recipient_name)
        .bind(&snapshot.phone_number)
        .bind(&snapshot.street_address)
        .bind(&snapshot.subdistrict)
        .bind(&snapshot.city)
        .bind(&snapshot.province)
        .bind(&snapshot.postal_code)
        .bind(&snapshot_json)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        // Insert order items
        for oi in &order_items {
            let item_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO order_items (id, order_id, product_id, product_name, quantity, unit_price, total_price)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(item_id.to_string())
            .bind(order_id.to_string())
            .bind(oi.product_id.to_string())
            .bind(&oi.product_name)
            .bind(oi.quantity as i64)
            .bind(oi.unit_price)
            .bind(oi.total_price)
            .execute(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;
        }

        tracing::info!(order_id = %order_id, total = %total_amount, "Storefront Order Created in database with immutable snapshot");

        if let Some(ref email_sender) = self.email_sender {
            let email_to = req.customer_email.trim().to_string();
            if !email_to.is_empty() && email_to.contains('@') {
                let sender = email_sender.clone();
                let oid_str = order_id.to_string();
                let c_name = req.customer_name.trim().to_string();
                let s_addr = req.shipping_address.trim().to_string();
                let s_name = self.store_name.clone();
                let email_items: Vec<program1_core::OrderEmailItem> = order_items
                    .iter()
                    .map(|it| program1_core::OrderEmailItem {
                        name: it.product_name.clone(),
                        quantity: it.quantity,
                        price: it.unit_price,
                    })
                    .collect();

                tokio::spawn(async move {
                    let (subject, body) = program1_core::order_confirmation_email(
                        &oid_str,
                        &c_name,
                        total_amount,
                        &email_items,
                        &s_addr,
                        &s_name,
                    );
                    if let Err(err) = sender.send_email(&email_to, &subject, &body).await {
                        tracing::error!(target: "email", "Failed to send order confirmation email to {}: {}", email_to, err);
                    } else {
                        tracing::info!(target: "email", "Order confirmation email sent successfully to {}", email_to);
                    }
                });
            }
        }

        Ok(OmniOrderDto {
            id: order_id,
            channel: ChannelType::NativeWeb,
            customer_name: req.customer_name.trim().to_string(),
            customer_email: req.customer_email.trim().to_string(),
            shipping_address: req.shipping_address.trim().to_string(),
            items: order_items,
            total_amount,
            status: status_str.to_string(),
            created_at: now,
            buyer_id: req.buyer_id,
            shipping_snapshot: Some(snapshot),
            tracking_number: None,
            shipped_at: None,
            delivered_at: None,
            cancelled_at: None,
            cancelled_by: None,
            cancel_reason: None,
        })
    }

    async fn create_marketplace_order(
        &self,
        channel: ChannelType,
        customer_name: String,
        items: Vec<StorefrontOrderItemRequest>,
    ) -> Result<OmniOrderDto, ContractError> {
        let mut order_items = Vec::new();
        let mut total_amount = 0.0;

        for item in &items {
            let product = self.catalog_contract.get_item(item.product_id).await?;
            self.inventory_contract
                .reserve_stock(item.product_id, item.quantity)
                .await?;

            let item_total = product.price * (item.quantity as f64);
            total_amount += item_total;

            order_items.push(OrderItemDto {
                product_id: product.id,
                product_name: product.name,
                quantity: item.quantity,
                unit_price: product.price,
                total_price: item_total,
            });
        }

        let order_id = Uuid::new_v4();
        let now = Utc::now();
        let ch_str = channel.to_string();
        let status_str = "processing";

        sqlx::query(
            "INSERT INTO orders (id, channel, customer_name, customer_email, shipping_address, total_amount, status, created_at, buyer_id, shipping_recipient_name, shipping_phone_number, shipping_street_address, shipping_subdistrict, shipping_city, shipping_province, shipping_postal_code, shipping_snapshot_json)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULL, $9, '', $10, '', '', '', '', '')",
        )
        .bind(order_id.to_string())
        .bind(&ch_str)
        .bind(customer_name.trim())
        .bind("marketplace_buyer@auto.com")
        .bind("Marketplace Logistics Address")
        .bind(total_amount)
        .bind(status_str)
        .bind(now.to_rfc3339())
        .bind(customer_name.trim())
        .bind("Marketplace Logistics Address")
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        for oi in &order_items {
            let item_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO order_items (id, order_id, product_id, product_name, quantity, unit_price, total_price)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(item_id.to_string())
            .bind(order_id.to_string())
            .bind(oi.product_id.to_string())
            .bind(&oi.product_name)
            .bind(oi.quantity as i64)
            .bind(oi.unit_price)
            .bind(oi.total_price)
            .execute(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;
        }

        Ok(OmniOrderDto {
            id: order_id,
            channel,
            customer_name,
            customer_email: "marketplace_buyer@auto.com".to_string(),
            shipping_address: "Marketplace Logistics Address".to_string(),
            items: order_items,
            total_amount,
            status: status_str.to_string(),
            created_at: now,
            buyer_id: None,
            shipping_snapshot: None,
            tracking_number: None,
            shipped_at: None,
            delivered_at: None,
            cancelled_at: None,
            cancelled_by: None,
            cancel_reason: None,
        })
    }

    async fn list_orders(&self) -> Result<Vec<OmniOrderDto>, ContractError> {
        let rows = sqlx::query(
            "SELECT id, channel, customer_name, customer_email, shipping_address, total_amount, status, created_at, buyer_id, shipping_snapshot_json, tracking_number, shipped_at, delivered_at, cancelled_at, cancelled_by, cancel_reason
             FROM orders ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let mut list = Vec::new();
        for r in rows {
            let id_str: String = r.get("id");
            let items = self.fetch_items_for_order(&id_str).await?;
            list.push(self.row_to_order_dto(&r, items)?);
        }

        Ok(list)
    }

    async fn list_orders_paginated(
        &self,
        page: i64,
        page_size: i64,
        status: Option<&str>,
        sort_by: Option<&str>,
        sort_order: Option<&str>,
    ) -> Result<PaginatedResponse<OmniOrderDto>, ContractError> {
        let page = page.max(1);
        let page_size = page_size.clamp(1, 100);
        let offset = (page - 1) * page_size;

        let status_term = status.map(|s| s.trim()).filter(|s| !s.is_empty());

        let order_col = match sort_by.unwrap_or("created_at").to_lowercase().as_str() {
            "total_amount" => "total_amount",
            "status" => "status",
            _ => "created_at",
        };
        let order_dir = match sort_order.unwrap_or("desc").to_lowercase().as_str() {
            "asc" => "ASC",
            _ => "DESC",
        };

        let count_row = sqlx::query(
            "SELECT COUNT(*) as total FROM orders
             WHERE ($1 IS NULL OR status = $1)",
        )
        .bind(status_term)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let total: i64 = count_row.get("total");

        let query_str = format!(
            "SELECT id, channel, customer_name, customer_email, shipping_address, total_amount, status, created_at, buyer_id, shipping_snapshot_json, tracking_number, shipped_at, delivered_at, cancelled_at, cancelled_by, cancel_reason
             FROM orders
             WHERE ($1 IS NULL OR status = $1)
             ORDER BY {} {}
             LIMIT $2 OFFSET $3",
            order_col, order_dir
        );

        let rows = sqlx::query(&query_str)
            .bind(status_term)
            .bind(page_size)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let mut data = Vec::new();
        for r in rows {
            let id_str: String = r.get("id");
            let items = self.fetch_items_for_order(&id_str).await?;
            data.push(self.row_to_order_dto(&r, items)?);
        }

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

    async fn update_order_status(
        &self,
        order_id: Uuid,
        new_status: OrderStatus,
        updated_by: &str,
    ) -> Result<OmniOrderDto, ContractError> {
        self.update_order_status_with_metadata(order_id, new_status, updated_by, None, None)
            .await
    }

    async fn update_order_status_with_metadata(
        &self,
        order_id: Uuid,
        new_status: OrderStatus,
        updated_by: &str,
        tracking_number: Option<String>,
        reason: Option<String>,
    ) -> Result<OmniOrderDto, ContractError> {
        let order = self.get_order(order_id).await?;
        let current_status = OrderStatus::from_str(&order.status).unwrap_or(OrderStatus::Pending);

        if !current_status.can_transition_to(&new_status) {
            return Err(ContractError::ValidationError(format!(
                "Transisi status pesanan tidak valid: tidak dapat mengubah dari '{}' ke '{}'",
                current_status.as_str(),
                new_status.as_str()
            )));
        }

        let now = Utc::now();
        let now_str = now.to_rfc3339();

        let (
            shipped_at_update,
            delivered_at_update,
            cancelled_at_update,
            cancelled_by_update,
            cancel_reason_update,
        ) = match new_status {
            OrderStatus::Shipped => (Some(now_str), None, None, None, None),
            OrderStatus::Delivered => (None, Some(now_str), None, None, None),
            OrderStatus::Cancelled => (
                None,
                None,
                Some(now_str),
                Some(updated_by.to_string()),
                reason.clone(),
            ),
            _ => (None, None, None, None, None),
        };

        sqlx::query(
            "UPDATE orders SET
                status = $1,
                tracking_number = COALESCE($2, tracking_number),
                shipped_at = COALESCE($3, shipped_at),
                delivered_at = COALESCE($4, delivered_at),
                cancelled_at = COALESCE($5, cancelled_at),
                cancelled_by = COALESCE($6, cancelled_by),
                cancel_reason = COALESCE($7, cancel_reason)
             WHERE id = $8",
        )
        .bind(new_status.as_str())
        .bind(&tracking_number)
        .bind(&shipped_at_update)
        .bind(&delivered_at_update)
        .bind(&cancelled_at_update)
        .bind(&cancelled_by_update)
        .bind(&cancel_reason_update)
        .bind(order_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let updated_order = self.get_order(order_id).await?;

        if let Some(ref email_sender) = self.email_sender {
            let email_to = updated_order.customer_email.trim().to_string();
            if !email_to.is_empty() && email_to.contains('@') {
                let sender = email_sender.clone();
                let oid_str = order_id.to_string();
                let c_name = updated_order.customer_name.clone();
                let s_name = self.store_name.clone();

                match new_status {
                    OrderStatus::Paid => {
                        let amt = updated_order.total_amount;
                        tokio::spawn(async move {
                            let (subject, body) = program1_core::payment_success_email(
                                &oid_str,
                                &c_name,
                                "Pembayaran Terkonfirmasi",
                                amt,
                                &s_name,
                            );
                            if let Err(err) = sender.send_email(&email_to, &subject, &body).await {
                                tracing::error!(target: "email", "Failed to send payment success email to {}: {}", email_to, err);
                            } else {
                                tracing::info!(target: "email", "Payment success email sent to {}", email_to);
                            }
                        });
                    }
                    OrderStatus::Shipped => {
                        let t_num = updated_order.tracking_number.clone();
                        tokio::spawn(async move {
                            let (subject, body) = program1_core::shipping_notification_email(
                                &oid_str,
                                &c_name,
                                t_num.as_deref(),
                                Some("Kurir Ekspedisi"),
                                &s_name,
                            );
                            if let Err(err) = sender.send_email(&email_to, &subject, &body).await {
                                tracing::error!(target: "email", "Failed to send shipping notification email to {}: {}", email_to, err);
                            } else {
                                tracing::info!(target: "email", "Shipping notification email sent to {}", email_to);
                            }
                        });
                    }
                    _ => {}
                }
            }
        }

        Ok(updated_order)
    }

    async fn list_buyer_orders(&self, buyer_id: Uuid) -> Result<Vec<OmniOrderDto>, ContractError> {
        let rows = sqlx::query(
            "SELECT id, channel, customer_name, customer_email, shipping_address, total_amount, status, created_at, buyer_id, shipping_snapshot_json, tracking_number, shipped_at, delivered_at, cancelled_at, cancelled_by, cancel_reason
             FROM orders WHERE buyer_id = $1 ORDER BY created_at DESC",
        )
        .bind(buyer_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let mut list = Vec::new();
        for r in rows {
            let id_str: String = r.get("id");
            let items = self.fetch_items_for_order(&id_str).await?;
            list.push(self.row_to_order_dto(&r, items)?);
        }

        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use program1_core::init_database;
    use program1_module_catalog::CatalogModule;
    use program1_module_inventory::InventoryModule;

    #[tokio::test]
    async fn test_order_creation_contract() {
        let pool = init_database("sqlite::memory:").await.unwrap();
        let catalog = Arc::new(CatalogModule::new(pool.clone()));
        catalog.seed_default_catalog().await.unwrap();
        let inventory = Arc::new(InventoryModule::new(pool.clone(), catalog.clone()));
        let order_module = OrderModule::new(pool, catalog.clone(), inventory.clone());

        let products = catalog.list_items().await.unwrap();
        let req = StorefrontOrderRequest {
            customer_name: "Customer Jane".to_string(),
            customer_email: "jane@test.com".to_string(),
            shipping_address: "Jakarta Selatan".to_string(),
            items: vec![StorefrontOrderItemRequest {
                product_id: products[0].id,
                quantity: 2,
            }],
            buyer_id: None,
            shipping_snapshot: None,
        };

        let order = order_module.create_storefront_order(req).await.unwrap();
        assert_eq!(order.customer_name, "Customer Jane");
        assert_eq!(order.channel, ChannelType::NativeWeb);
        assert_eq!(order.items.len(), 1);

        let fetched = order_module.get_order(order.id).await.unwrap();
        assert_eq!(fetched.total_amount, order.total_amount);
    }

    #[tokio::test]
    async fn test_order_shipping_snapshot_and_buyer_link() {
        let pool = init_database("sqlite::memory:").await.unwrap();
        let catalog = Arc::new(CatalogModule::new(pool.clone()));
        catalog.seed_default_catalog().await.unwrap();
        let inventory = Arc::new(InventoryModule::new(pool.clone(), catalog.clone()));
        let order_module = OrderModule::new(pool, catalog.clone(), inventory.clone());

        let products = catalog.list_items().await.unwrap();
        let buyer_id = Uuid::new_v4();
        let snapshot = ShippingAddressSnapshot {
            recipient_name: "Budi Buyer".to_string(),
            phone_number: "+628123456789".to_string(),
            street_address: "Jl. Sudirman Kav 52-53".to_string(),
            subdistrict: "Senayan".to_string(),
            city: "Jakarta Selatan".to_string(),
            province: "DKI Jakarta".to_string(),
            postal_code: "12190".to_string(),
        };

        let req = StorefrontOrderRequest {
            customer_name: "Budi Buyer".to_string(),
            customer_email: "budi@buyer.com".to_string(),
            shipping_address: "Jl. Sudirman Kav 52-53, Senayan, Jakarta Selatan, DKI Jakarta 12190"
                .to_string(),
            items: vec![StorefrontOrderItemRequest {
                product_id: products[0].id,
                quantity: 1,
            }],
            buyer_id: Some(buyer_id),
            shipping_snapshot: Some(snapshot.clone()),
        };

        let order = order_module.create_storefront_order(req).await.unwrap();
        assert_eq!(order.buyer_id, Some(buyer_id));
        assert!(order.shipping_snapshot.is_some());
        let saved_snap = order.shipping_snapshot.unwrap();
        assert_eq!(saved_snap.recipient_name, "Budi Buyer");
        assert_eq!(saved_snap.phone_number, "+628123456789");
        assert_eq!(saved_snap.postal_code, "12190");

        let fetched = order_module.get_order(order.id).await.unwrap();
        assert_eq!(fetched.buyer_id, Some(buyer_id));
        let fetched_snap = fetched.shipping_snapshot.expect("Snapshot must persist");
        assert_eq!(fetched_snap.street_address, "Jl. Sudirman Kav 52-53");
    }

    #[tokio::test]
    async fn test_order_status_transitions_lifecycle() {
        let pool = init_database("sqlite::memory:").await.unwrap();
        let catalog = Arc::new(CatalogModule::new(pool.clone()));
        catalog.seed_default_catalog().await.unwrap();
        let inventory = Arc::new(InventoryModule::new(pool.clone(), catalog.clone()));
        let order_module = OrderModule::new(pool, catalog.clone(), inventory.clone());

        let products = catalog.list_items().await.unwrap();
        let req = StorefrontOrderRequest {
            customer_name: "Customer Lifecycle".to_string(),
            customer_email: "lifecycle@test.com".to_string(),
            shipping_address: "Jl. Merdeka 1".to_string(),
            items: vec![StorefrontOrderItemRequest {
                product_id: products[0].id,
                quantity: 1,
            }],
            buyer_id: None,
            shipping_snapshot: None,
        };

        let order = order_module.create_storefront_order(req).await.unwrap();
        assert_eq!(order.status, "pending");

        // 1. pending -> paid
        let paid_order = order_module
            .update_order_status(order.id, OrderStatus::Paid, "system_midtrans")
            .await
            .unwrap();
        assert_eq!(paid_order.status, "paid");

        // 2. paid -> processing
        let proc_order = order_module
            .update_order_status(order.id, OrderStatus::Processing, "seller_admin")
            .await
            .unwrap();
        assert_eq!(proc_order.status, "processing");

        // 3. processing -> shipped with tracking
        let shipped_order = order_module
            .update_order_status_with_metadata(
                order.id,
                OrderStatus::Shipped,
                "seller_admin",
                Some("JNT123456789".to_string()),
                None,
            )
            .await
            .unwrap();
        assert_eq!(shipped_order.status, "shipped");
        assert_eq!(
            shipped_order.tracking_number,
            Some("JNT123456789".to_string())
        );
        assert!(shipped_order.shipped_at.is_some());

        // 4. shipped -> delivered
        let deliv_order = order_module
            .update_order_status(order.id, OrderStatus::Delivered, "buyer")
            .await
            .unwrap();
        assert_eq!(deliv_order.status, "delivered");
        assert!(deliv_order.delivered_at.is_some());

        // 5. delivered -> completed
        let comp_order = order_module
            .update_order_status(order.id, OrderStatus::Completed, "system_auto")
            .await
            .unwrap();
        assert_eq!(comp_order.status, "completed");
    }

    #[tokio::test]
    async fn test_invalid_order_status_transitions() {
        let pool = init_database("sqlite::memory:").await.unwrap();
        let catalog = Arc::new(CatalogModule::new(pool.clone()));
        catalog.seed_default_catalog().await.unwrap();
        let inventory = Arc::new(InventoryModule::new(pool.clone(), catalog.clone()));
        let order_module = OrderModule::new(pool, catalog.clone(), inventory.clone());

        let products = catalog.list_items().await.unwrap();
        let req = StorefrontOrderRequest {
            customer_name: "Customer Invalid".to_string(),
            customer_email: "invalid@test.com".to_string(),
            shipping_address: "Jl. Merdeka 2".to_string(),
            items: vec![StorefrontOrderItemRequest {
                product_id: products[0].id,
                quantity: 1,
            }],
            buyer_id: None,
            shipping_snapshot: None,
        };

        let order = order_module.create_storefront_order(req).await.unwrap();
        assert_eq!(order.status, "pending");

        // Invalid: pending -> shipped directly
        let err = order_module
            .update_order_status(order.id, OrderStatus::Shipped, "seller")
            .await
            .unwrap_err();
        assert!(matches!(err, ContractError::ValidationError(_)));

        // Invalid: pending -> completed directly
        let err2 = order_module
            .update_order_status(order.id, OrderStatus::Completed, "seller")
            .await
            .unwrap_err();
        assert!(matches!(err2, ContractError::ValidationError(_)));
    }

    #[tokio::test]
    async fn test_buyer_order_cancellation_and_history() {
        let pool = init_database("sqlite::memory:").await.unwrap();
        let catalog = Arc::new(CatalogModule::new(pool.clone()));
        catalog.seed_default_catalog().await.unwrap();
        let inventory = Arc::new(InventoryModule::new(pool.clone(), catalog.clone()));
        let order_module = OrderModule::new(pool, catalog.clone(), inventory.clone());

        let products = catalog.list_items().await.unwrap();
        let buyer_id = Uuid::new_v4();

        let req = StorefrontOrderRequest {
            customer_name: "Customer Cancel".to_string(),
            customer_email: "cancel@test.com".to_string(),
            shipping_address: "Jl. Merdeka 3".to_string(),
            items: vec![StorefrontOrderItemRequest {
                product_id: products[0].id,
                quantity: 1,
            }],
            buyer_id: Some(buyer_id),
            shipping_snapshot: None,
        };

        let order = order_module.create_storefront_order(req).await.unwrap();
        assert_eq!(order.status, "pending");

        // Buyer cancels pending order
        let cancelled = order_module
            .update_order_status_with_metadata(
                order.id,
                OrderStatus::Cancelled,
                "buyer",
                None,
                Some("Salah pilih item".to_string()),
            )
            .await
            .unwrap();
        assert_eq!(cancelled.status, "cancelled");
        assert_eq!(cancelled.cancelled_by, Some("buyer".to_string()));
        assert_eq!(
            cancelled.cancel_reason,
            Some("Salah pilih item".to_string())
        );
        assert!(cancelled.cancelled_at.is_some());

        // Cancelled order cannot transition to processing
        let err = order_module
            .update_order_status(order.id, OrderStatus::Processing, "seller")
            .await
            .unwrap_err();
        assert!(matches!(err, ContractError::ValidationError(_)));

        // List buyer orders returns the cancelled order
        let buyer_orders = order_module.list_buyer_orders(buyer_id).await.unwrap();
        assert_eq!(buyer_orders.len(), 1);
        assert_eq!(buyer_orders[0].id, order.id);
        assert_eq!(buyer_orders[0].status, "cancelled");
    }
}
