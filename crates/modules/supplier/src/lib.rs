use async_trait::async_trait;
use chrono::Utc;
use program1_contracts::*;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

pub struct SupplierModule {
    pool: SqlitePool,
}

impl SupplierModule {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SupplierContract for SupplierModule {
    async fn list_suppliers(&self) -> Result<Vec<SupplierDto>, ContractError> {
        let rows = sqlx::query(
            "SELECT id, name, contact_person, phone, email, address, is_active, created_at, updated_at
             FROM suppliers ORDER BY name ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let mut list = Vec::new();
        for r in rows {
            let is_active_int: i64 = r.get("is_active");
            list.push(SupplierDto {
                id: r.get("id"),
                name: r.get("name"),
                contact_person: r.get("contact_person"),
                phone: r.get("phone"),
                email: r.get("email"),
                address: r.get("address"),
                is_active: is_active_int != 0,
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            });
        }
        Ok(list)
    }

    async fn get_supplier(&self, id: &str) -> Result<SupplierDto, ContractError> {
        let row = sqlx::query(
            "SELECT id, name, contact_person, phone, email, address, is_active, created_at, updated_at
             FROM suppliers WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        match row {
            Some(r) => {
                let is_active_int: i64 = r.get("is_active");
                Ok(SupplierDto {
                    id: r.get("id"),
                    name: r.get("name"),
                    contact_person: r.get("contact_person"),
                    phone: r.get("phone"),
                    email: r.get("email"),
                    address: r.get("address"),
                    is_active: is_active_int != 0,
                    created_at: r.get("created_at"),
                    updated_at: r.get("updated_at"),
                })
            }
            None => Err(ContractError::NotFound(format!("Supplier dengan ID {} tidak ditemukan", id))),
        }
    }

    async fn create_supplier(&self, req: CreateSupplierRequest) -> Result<SupplierDto, ContractError> {
        let name = req.name.trim();
        if name.is_empty() {
            return Err(ContractError::ValidationError("Nama supplier tidak boleh kosong".to_string()));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO suppliers (id, name, contact_person, phone, email, address, is_active, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, 1, $7, $8)",
        )
        .bind(&id)
        .bind(name)
        .bind(req.contact_person.as_deref().map(str::trim))
        .bind(req.phone.as_deref().map(str::trim))
        .bind(req.email.as_deref().map(str::trim))
        .bind(req.address.as_deref().map(str::trim))
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        self.get_supplier(&id).await
    }

    async fn update_supplier(&self, id: &str, req: UpdateSupplierRequest) -> Result<SupplierDto, ContractError> {
        let _ = self.get_supplier(id).await?;
        let name = req.name.trim();
        if name.is_empty() {
            return Err(ContractError::ValidationError("Nama supplier tidak boleh kosong".to_string()));
        }

        let now = Utc::now().to_rfc3339();
        let is_active_val = req.is_active.map(|b| if b { 1i64 } else { 0i64 });

        sqlx::query(
            "UPDATE suppliers SET 
                name = $1,
                contact_person = $2,
                phone = $3,
                email = $4,
                address = $5,
                is_active = COALESCE($6, is_active),
                updated_at = $7
             WHERE id = $8",
        )
        .bind(name)
        .bind(req.contact_person.as_deref().map(str::trim))
        .bind(req.phone.as_deref().map(str::trim))
        .bind(req.email.as_deref().map(str::trim))
        .bind(req.address.as_deref().map(str::trim))
        .bind(is_active_val)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        self.get_supplier(id).await
    }

    async fn delete_supplier(&self, id: &str) -> Result<(), ContractError> {
        let _ = self.get_supplier(id).await?;
        let count_row = sqlx::query("SELECT COUNT(*) as count FROM purchase_orders WHERE supplier_id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let count: i64 = count_row.get("count");
        if count > 0 {
            return Err(ContractError::ValidationError(
                "Supplier tidak dapat dihapus karena masih memiliki riwayat purchase order".to_string(),
            ));
        }

        sqlx::query("DELETE FROM suppliers WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn list_purchase_orders(&self, status_filter: Option<&str>) -> Result<Vec<PurchaseOrderDto>, ContractError> {
        let mut query = String::from(
            "SELECT p.id, p.po_number, p.supplier_id, s.name as supplier_name, p.status, p.total_cost_cents, p.expected_delivery_date, p.notes, p.created_at, p.updated_at
             FROM purchase_orders p
             JOIN suppliers s ON s.id = p.supplier_id",
        );

        if let Some(status) = status_filter {
            if !status.is_empty() {
                query.push_str(&format!(" WHERE p.status = '{}'", status.trim().to_lowercase()));
            }
        }
        query.push_str(" ORDER BY p.created_at DESC");

        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let mut list = Vec::new();
        for r in rows {
            let po_id: String = r.get("id");
            let items = self.fetch_items_for_po(&po_id).await?;

            list.push(PurchaseOrderDto {
                id: po_id,
                po_number: r.get("po_number"),
                supplier_id: r.get("supplier_id"),
                supplier_name: r.get("supplier_name"),
                status: r.get("status"),
                total_cost_cents: r.get("total_cost_cents"),
                expected_delivery_date: r.get("expected_delivery_date"),
                notes: r.get("notes"),
                items,
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            });
        }
        Ok(list)
    }

    async fn get_purchase_order(&self, id: &str) -> Result<PurchaseOrderDto, ContractError> {
        let row = sqlx::query(
            "SELECT p.id, p.po_number, p.supplier_id, s.name as supplier_name, p.status, p.total_cost_cents, p.expected_delivery_date, p.notes, p.created_at, p.updated_at
             FROM purchase_orders p
             JOIN suppliers s ON s.id = p.supplier_id
             WHERE p.id = $1 OR p.po_number = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        match row {
            Some(r) => {
                let po_id: String = r.get("id");
                let items = self.fetch_items_for_po(&po_id).await?;

                Ok(PurchaseOrderDto {
                    id: po_id,
                    po_number: r.get("po_number"),
                    supplier_id: r.get("supplier_id"),
                    supplier_name: r.get("supplier_name"),
                    status: r.get("status"),
                    total_cost_cents: r.get("total_cost_cents"),
                    expected_delivery_date: r.get("expected_delivery_date"),
                    notes: r.get("notes"),
                    items,
                    created_at: r.get("created_at"),
                    updated_at: r.get("updated_at"),
                })
            }
            None => Err(ContractError::NotFound(format!("Purchase order {} tidak ditemukan", id))),
        }
    }

    async fn create_purchase_order(&self, req: CreatePurchaseOrderRequest) -> Result<PurchaseOrderDto, ContractError> {
        let supplier = self.get_supplier(&req.supplier_id).await?;
        if !supplier.is_active {
            return Err(ContractError::ValidationError("Supplier sedang nonaktif".to_string()));
        }

        if req.items.is_empty() {
            return Err(ContractError::ValidationError("Minimal harus ada 1 item dalam purchase order".to_string()));
        }

        let po_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let po_number = format!("PO-{}-{}", now.format("%Y%m%d"), &po_id[..6].to_uppercase());
        let now_str = now.to_rfc3339();

        let mut total_cost_cents: i64 = 0;
        for item in &req.items {
            if item.quantity <= 0 {
                return Err(ContractError::ValidationError("Jumlah item harus lebih dari 0".to_string()));
            }
            if item.unit_cost_cents < 0 {
                return Err(ContractError::ValidationError("Biaya unit tidak boleh negatif".to_string()));
            }
            total_cost_cents += item.quantity * item.unit_cost_cents;
        }

        let mut tx = self.pool.begin().await.map_err(|e| ContractError::Internal(e.to_string()))?;

        sqlx::query(
            "INSERT INTO purchase_orders (id, po_number, supplier_id, status, total_cost_cents, expected_delivery_date, notes, created_at, updated_at)
             VALUES ($1, $2, $3, 'ordered', $4, $5, $6, $7, $8)",
        )
        .bind(&po_id)
        .bind(&po_number)
        .bind(&req.supplier_id)
        .bind(total_cost_cents)
        .bind(req.expected_delivery_date.as_deref())
        .bind(req.notes.as_deref().map(str::trim))
        .bind(&now_str)
        .bind(&now_str)
        .execute(&mut *tx)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        for item in &req.items {
            let item_id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO purchase_order_items (id, po_id, product_id, variant_id, quantity, unit_cost_cents, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(&item_id)
            .bind(&po_id)
            .bind(&item.product_id)
            .bind(item.variant_id.as_deref())
            .bind(item.quantity)
            .bind(item.unit_cost_cents)
            .bind(&now_str)
            .execute(&mut *tx)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;
        }

        tx.commit().await.map_err(|e| ContractError::Internal(e.to_string()))?;

        self.get_purchase_order(&po_id).await
    }

    async fn receive_purchase_order(&self, id: &str) -> Result<PurchaseOrderDto, ContractError> {
        let po = self.get_purchase_order(id).await?;

        if po.status == "received" {
            return Err(ContractError::ValidationError("Purchase Order sudah diterima sebelumnya".to_string()));
        }
        if po.status == "cancelled" {
            return Err(ContractError::ValidationError("Purchase Order yang dibatalkan tidak dapat diterima".to_string()));
        }

        let now_str = Utc::now().to_rfc3339();
        let mut tx = self.pool.begin().await.map_err(|e| ContractError::Internal(e.to_string()))?;

        // Process restock for each item
        for item in &po.items {
            // 1. Update stock in catalog_items
            sqlx::query("UPDATE catalog_items SET stock = stock + $1 WHERE id = $2")
                .bind(item.quantity)
                .bind(&item.product_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| ContractError::Internal(e.to_string()))?;

            // 2. Update stock in inventory_stocks if present
            let inv_row = sqlx::query("SELECT warehouse_stock, available_stock FROM inventory_stocks WHERE product_id = $1")
                .bind(&item.product_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| ContractError::Internal(e.to_string()))?;

            if let Some(r) = inv_row {
                let old_warehouse: i64 = r.get("warehouse_stock");
                let new_warehouse = old_warehouse + item.quantity;

                sqlx::query(
                    "UPDATE inventory_stocks SET 
                        warehouse_stock = warehouse_stock + $1,
                        available_stock = available_stock + $1,
                        last_updated = $2
                     WHERE product_id = $3",
                )
                .bind(item.quantity)
                .bind(&now_str)
                .bind(&item.product_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| ContractError::Internal(e.to_string()))?;

                // 3. Log stock adjustment
                let log_id = Uuid::new_v4().to_string();
                let admin_note = format!("Penerimaan Barang Supplier PO #{}", po.po_number);
                let _ = sqlx::query(
                    "INSERT INTO stock_adjustment_logs (id, product_id, adjustment_type, old_value, new_value, admin_note, updated_by, timestamp)
                     VALUES ($1, $2, 'warehouse', $3, $4, $5, 'SupplierPO', $6)",
                )
                .bind(&log_id)
                .bind(&item.product_id)
                .bind(old_warehouse)
                .bind(new_warehouse)
                .bind(&admin_note)
                .bind(&now_str)
                .execute(&mut *tx)
                .await;
            }
        }

        // 4. Update status to received
        sqlx::query("UPDATE purchase_orders SET status = 'received', updated_at = $1 WHERE id = $2")
            .bind(&now_str)
            .bind(&po.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        tx.commit().await.map_err(|e| ContractError::Internal(e.to_string()))?;

        self.get_purchase_order(&po.id).await
    }

    async fn cancel_purchase_order(&self, id: &str) -> Result<PurchaseOrderDto, ContractError> {
        let po = self.get_purchase_order(id).await?;

        if po.status == "received" {
            return Err(ContractError::ValidationError("Purchase Order yang sudah diterima tidak dapat dibatalkan".to_string()));
        }
        if po.status == "cancelled" {
            return Ok(po);
        }

        let now_str = Utc::now().to_rfc3339();
        sqlx::query("UPDATE purchase_orders SET status = 'cancelled', updated_at = $1 WHERE id = $2")
            .bind(&now_str)
            .bind(&po.id)
            .execute(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        self.get_purchase_order(&po.id).await
    }
}

impl SupplierModule {
    async fn fetch_items_for_po(&self, po_id: &str) -> Result<Vec<PurchaseOrderItemDto>, ContractError> {
        let rows = sqlx::query(
            "SELECT poi.id, poi.po_id, poi.product_id, poi.variant_id, poi.quantity, poi.unit_cost_cents, c.name as product_name
             FROM purchase_order_items poi
             LEFT JOIN catalog_items c ON c.id = poi.product_id
             WHERE poi.po_id = $1",
        )
        .bind(po_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let mut items = Vec::new();
        for r in rows {
            items.push(PurchaseOrderItemDto {
                id: r.get("id"),
                po_id: r.get("po_id"),
                product_id: r.get("product_id"),
                product_name: r.get("product_name"),
                variant_id: r.get("variant_id"),
                quantity: r.get("quantity"),
                unit_cost_cents: r.get("unit_cost_cents"),
            });
        }
        Ok(items)
    }
}
