use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use program1_contracts::{
    AuditContract, AuditLogEntry, ContractError, CreateReturnRequest, InventoryContract,
    NotificationContract, ProcessReturnRequest, ReturnContract, ReturnRequestDto,
    UpdateReturnStatusRequest,
};

pub struct ReturnModule {
    pool: SqlitePool,
    inventory_contract: Option<Arc<dyn InventoryContract>>,
    notification_contract: Option<Arc<dyn NotificationContract>>,
    audit_contract: Option<Arc<dyn AuditContract>>,
}

impl ReturnModule {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            inventory_contract: None,
            notification_contract: None,
            audit_contract: None,
        }
    }

    pub fn with_inventory_contract(mut self, contract: Arc<dyn InventoryContract>) -> Self {
        self.inventory_contract = Some(contract);
        self
    }

    pub fn with_notification_contract(mut self, contract: Arc<dyn NotificationContract>) -> Self {
        self.notification_contract = Some(contract);
        self
    }

    pub fn with_audit_contract(mut self, contract: Arc<dyn AuditContract>) -> Self {
        self.audit_contract = Some(contract);
        self
    }

    fn row_to_dto(row: &sqlx::sqlite::SqliteRow) -> ReturnRequestDto {
        let evidence_raw: String = row.get("evidence_urls");
        let evidence_urls: Vec<String> = serde_json::from_str(&evidence_raw).unwrap_or_default();

        ReturnRequestDto {
            id: row.get("id"),
            order_id: row.get("order_id"),
            buyer_id: row.get("buyer_id"),
            reason: row.get("reason"),
            description: row.get("description"),
            evidence_urls,
            status: row.get("status"),
            refund_amount_cents: row.get("refund_amount_cents"),
            admin_notes: row.get("admin_notes"),
            processed_by: row.get("processed_by"),
            processed_at: row.get("processed_at"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

#[async_trait]
impl ReturnContract for ReturnModule {
    async fn create_return_request(
        &self,
        buyer_id: &str,
        req: CreateReturnRequest,
    ) -> Result<ReturnRequestDto, ContractError> {
        // 1. Verify order exists
        let order_row = sqlx::query("SELECT id, buyer_id, status, total_amount FROM orders WHERE id = $1")
            .bind(&req.order_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let order = match order_row {
            Some(row) => row,
            None => return Err(ContractError::NotFound("Pesanan tidak ditemukan".into())),
        };

        // 2. Verify order belongs to buyer
        let order_buyer_id: Option<String> = order.get("buyer_id");
        if order_buyer_id.as_deref() != Some(buyer_id) {
            return Err(ContractError::ValidationError("Anda tidak memiliki akses ke pesanan ini".into()));
        }

        // 3. Verify order status is "delivered" or "completed"
        let status: String = order.get("status");
        let status_lower = status.to_lowercase();
        if status_lower != "delivered" && status_lower != "completed" {
            return Err(ContractError::ValidationError(
                "Pengajuan retur hanya dapat dilakukan untuk pesanan yang sudah terkirim (delivered)".into(),
            ));
        }

        // 4. Verify no existing non-rejected return request for this order
        let existing = sqlx::query("SELECT id FROM return_requests WHERE order_id = $1 AND status != 'rejected'")
            .bind(&req.order_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        if existing.is_some() {
            return Err(ContractError::AlreadyExists(
                "Permintaan retur untuk pesanan ini sudah ada dan sedang diproses".into(),
            ));
        }

        // 5. Validate reason
        let valid_reasons = ["defective", "wrong_item", "not_as_described", "other"];
        if !valid_reasons.contains(&req.reason.as_str()) {
            return Err(ContractError::ValidationError("Alasan retur tidak valid".into()));
        }

        let total_amount: f64 = order.try_get::<f64, _>("total_amount").unwrap_or(0.0);
        let refund_amount_cents: i64 = (total_amount * 100.0) as i64;
        let evidence_json = serde_json::to_string(&req.evidence_urls.unwrap_or_default()).unwrap_or_else(|_| "[]".to_string());

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO return_requests (id, order_id, buyer_id, reason, description, evidence_urls, status, refund_amount_cents, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, 'pending', $7, $8, $9)",
        )
        .bind(&id)
        .bind(&req.order_id)
        .bind(buyer_id)
        .bind(&req.reason)
        .bind(&req.description)
        .bind(&evidence_json)
        .bind(refund_amount_cents)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let created_row = sqlx::query("SELECT * FROM return_requests WHERE id = $1")
            .bind(&id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let dto = Self::row_to_dto(&created_row);

        // Dispatches notification to Seller/Admin
        if let Some(notif) = &self.notification_contract {
            let _ = notif
                .create_notification(
                    "seller",
                    "admin",
                    "Pengajuan Retur Baru",
                    &format!("Pembeli mengajukan retur untuk pesanan #{}", &req.order_id),
                    "return",
                    Some(&id),
                    Some("return"),
                )
                .await;
        }

        // Audit log
        if let Some(audit) = &self.audit_contract {
            let _ = audit
                .log_action(AuditLogEntry {
                    id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    actor_id: Uuid::parse_str(buyer_id).ok(),
                    actor_username: buyer_id.to_string(),
                    action: "CREATE_RETURN_REQUEST".to_string(),
                    resource_type: "return_requests".to_string(),
                    resource_id: Uuid::parse_str(&id).ok(),
                    details: format!("Created return request for order {}", req.order_id),
                    ip_address: None,
                })
                .await;
        }

        Ok(dto)
    }

    async fn list_buyer_returns(
        &self,
        buyer_id: &str,
    ) -> Result<Vec<ReturnRequestDto>, ContractError> {
        let rows = sqlx::query("SELECT * FROM return_requests WHERE buyer_id = $1 ORDER BY created_at DESC")
            .bind(buyer_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        Ok(rows.iter().map(Self::row_to_dto).collect())
    }

    async fn get_return_by_order(
        &self,
        order_id: &str,
    ) -> Result<Option<ReturnRequestDto>, ContractError> {
        let row = sqlx::query("SELECT * FROM return_requests WHERE order_id = $1 AND status != 'rejected' ORDER BY created_at DESC LIMIT 1")
            .bind(order_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        Ok(row.as_ref().map(Self::row_to_dto))
    }

    async fn list_all_returns(
        &self,
        status_filter: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ReturnRequestDto>, ContractError> {
        let rows = if let Some(status) = status_filter {
            sqlx::query("SELECT * FROM return_requests WHERE status = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3")
                .bind(status)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
        } else {
            sqlx::query("SELECT * FROM return_requests ORDER BY created_at DESC LIMIT $1 OFFSET $2")
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
        }
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        Ok(rows.iter().map(Self::row_to_dto).collect())
    }

    async fn process_return(
        &self,
        return_id: &str,
        admin_id: &str,
        req: ProcessReturnRequest,
    ) -> Result<ReturnRequestDto, ContractError> {
        let row = sqlx::query("SELECT * FROM return_requests WHERE id = $1")
            .bind(return_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let return_req = match row {
            Some(r) => r,
            None => return Err(ContractError::NotFound("Permintaan retur tidak ditemukan".into())),
        };

        let current_status: String = return_req.get("status");
        if current_status != "pending" {
            return Err(ContractError::ValidationError(
                "Hanya permintaan retur berstatus 'pending' yang dapat diproses".into(),
            ));
        }

        let now = Utc::now().to_rfc3339();
        let new_status: String;
        let mut refund_amount: i64 = return_req.get("refund_amount_cents");

        match req.action.to_lowercase().as_str() {
            "approve" | "approved" => {
                new_status = "approved".to_string();
                if let Some(override_amt) = req.refund_amount_override {
                    refund_amount = override_amt;
                }
            }
            "reject" | "rejected" => {
                new_status = "rejected".to_string();
                if req.admin_notes.as_deref().unwrap_or("").trim().is_empty() {
                    return Err(ContractError::ValidationError(
                        "Catatan admin wajib diisi saat menolak pengajuan retur".into(),
                    ));
                }
            }
            _ => return Err(ContractError::ValidationError("Aksi harus 'approve' atau 'reject'".into())),
        }

        sqlx::query(
            "UPDATE return_requests SET status = $1, refund_amount_cents = $2, admin_notes = $3, processed_by = $4, processed_at = $5, updated_at = $6 WHERE id = $7",
        )
        .bind(&new_status)
        .bind(refund_amount)
        .bind(&req.admin_notes)
        .bind(admin_id)
        .bind(&now)
        .bind(&now)
        .bind(return_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let updated_row = sqlx::query("SELECT * FROM return_requests WHERE id = $1")
            .bind(return_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let dto = Self::row_to_dto(&updated_row);

        // Send Notification to Buyer
        if let Some(notif) = &self.notification_contract {
            let notif_title = if new_status == "approved" {
                "Pengajuan Retur Disetujui"
            } else {
                "Pengajuan Retur Ditolak"
            };
            let notif_body = if new_status == "approved" {
                format!("Pengajuan retur untuk pesanan #{} telah disetujui. Silakan kirimkan barang retur Anda.", dto.order_id)
            } else {
                format!("Pengajuan retur untuk pesanan #{} ditolak. Alasan: {}", dto.order_id, req.admin_notes.as_deref().unwrap_or("-"))
            };

            let _ = notif
                .create_notification(
                    "buyer",
                    &dto.buyer_id,
                    notif_title,
                    &notif_body,
                    "return",
                    Some(return_id),
                    Some("return"),
                )
                .await;
        }

        // Audit Log
        if let Some(audit) = &self.audit_contract {
            let _ = audit
                .log_action(AuditLogEntry {
                    id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    actor_id: Uuid::parse_str(admin_id).ok(),
                    actor_username: admin_id.to_string(),
                    action: "PROCESS_RETURN_REQUEST".to_string(),
                    resource_type: "return_requests".to_string(),
                    resource_id: Uuid::parse_str(return_id).ok(),
                    details: format!("Processed return request {} with action {}", return_id, new_status),
                    ip_address: None,
                })
                .await;
        }

        Ok(dto)
    }

    async fn update_return_status(
        &self,
        return_id: &str,
        admin_id: &str,
        req: UpdateReturnStatusRequest,
    ) -> Result<ReturnRequestDto, ContractError> {
        let row = sqlx::query("SELECT * FROM return_requests WHERE id = $1")
            .bind(return_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let return_req = match row {
            Some(r) => r,
            None => return Err(ContractError::NotFound("Permintaan retur tidak ditemukan".into())),
        };

        let new_status = req.status.to_lowercase();
        let valid_statuses = ["return_shipped", "received", "refunded"];
        if !valid_statuses.contains(&new_status.as_str()) {
            return Err(ContractError::ValidationError(
                "Status retur tidak valid (pilihan: return_shipped, received, refunded)".into(),
            ));
        }

        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE return_requests SET status = $1, admin_notes = COALESCE($2, admin_notes), updated_at = $3 WHERE id = $4",
        )
        .bind(&new_status)
        .bind(&req.admin_notes)
        .bind(&now)
        .bind(return_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let order_id: String = return_req.get("order_id");
        let buyer_id: String = return_req.get("buyer_id");

        // AUTOMATIC INVENTORY RESTOCK ON 'RECEIVED'
        if new_status == "received" {
            let items = sqlx::query("SELECT product_id, quantity FROM order_items WHERE order_id = $1")
                .bind(&order_id)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| ContractError::Internal(e.to_string()))?;

            for item in items {
                let product_id_str: String = item.get("product_id");
                let qty: i32 = item.get("quantity");
                if let Ok(pid) = Uuid::parse_str(&product_id_str) {
                    if let Some(inv) = &self.inventory_contract {
                        if let Ok(current_stock) = inv.get_stock(pid).await {
                            let new_wh_stock = current_stock.warehouse_stock + (qty as u32);
                            let _ = inv
                                .update_warehouse_stock(
                                    pid,
                                    new_wh_stock,
                                    format!("Restock dari retur barang pesanan #{}", order_id),
                                    admin_id.to_string(),
                                )
                                .await;
                        }
                    }
                    
                    // Direct DB catalog stock update
                    let _ = sqlx::query("UPDATE catalog_items SET stock = stock + $1 WHERE id = $2")
                        .bind(qty)
                        .bind(&product_id_str)
                        .execute(&self.pool)
                        .await;
                }
            }
        }

        let updated_row = sqlx::query("SELECT * FROM return_requests WHERE id = $1")
            .bind(return_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let dto = Self::row_to_dto(&updated_row);

        // Notify Buyer
        if let Some(notif) = &self.notification_contract {
            let (title, message) = match new_status.as_str() {
                "return_shipped" => (
                    "Barang Retur Dalam Pengiriman",
                    format!("Barang retur pesanan #{} dalam proses pengiriman.", order_id),
                ),
                "received" => (
                    "Barang Retur Diterima Toko",
                    format!("Barang retur pesanan #{} telah diterima oleh toko dan stok telah diperbarui.", order_id),
                ),
                "refunded" => (
                    "Dana Retur Dikembalikan",
                    format!("Pengembalian dana sebesar Rp {} untuk pesanan #{} telah diproses.", dto.refund_amount_cents / 100, order_id),
                ),
                _ => ("Update Retur", format!("Status retur pesanan #{}: {}", order_id, new_status)),
            };

            let _ = notif
                .create_notification(
                    "buyer",
                    &buyer_id,
                    title,
                    &message,
                    "return",
                    Some(return_id),
                    Some("return"),
                )
                .await;
        }

        // Audit Log
        if let Some(audit) = &self.audit_contract {
            let _ = audit
                .log_action(AuditLogEntry {
                    id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    actor_id: Uuid::parse_str(admin_id).ok(),
                    actor_username: admin_id.to_string(),
                    action: "UPDATE_RETURN_STATUS".to_string(),
                    resource_type: "return_requests".to_string(),
                    resource_id: Uuid::parse_str(return_id).ok(),
                    details: format!("Updated return status for {} to {}", return_id, new_status),
                    ip_address: None,
                })
                .await;
        }

        Ok(dto)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_db() -> SqlitePool {
        let pool = program1_core::init_database("sqlite::memory:")
            .await
            .unwrap();
        pool
    }

    #[tokio::test]
    async fn test_return_module_basic() {
        let pool = setup_db().await;
        let module = ReturnModule::new(pool.clone());

        let buyer_id = Uuid::new_v4().to_string();
        let order_id = Uuid::new_v4().to_string();

        // Seed buyer account
        sqlx::query(
            "INSERT INTO buyer_accounts (id, email, full_name, created_at, updated_at) VALUES ($1, 'buyer@test.com', 'Buyer Test', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        )
        .bind(&buyer_id)
        .execute(&pool)
        .await
        .unwrap();

        // Seed order
        sqlx::query(
            "INSERT INTO orders (id, buyer_id, customer_name, status, total_amount) VALUES ($1, $2, 'Buyer Test', 'delivered', 150000.0)",
        )
        .bind(&order_id)
        .bind(&buyer_id)
        .execute(&pool)
        .await
        .unwrap();

        // Create return request
        let req = CreateReturnRequest {
            order_id: order_id.clone(),
            reason: "defective".to_string(),
            description: Some("Layar laptop retak".to_string()),
            evidence_urls: Some(vec!["https://example.com/photo.jpg".to_string()]),
        };

        let ret = module.create_return_request(&buyer_id, req).await.unwrap();
        assert_eq!(ret.order_id, order_id);
        assert_eq!(ret.status, "pending");
        assert_eq!(ret.refund_amount_cents, 15000000);

        // Process return - Approve
        let process_req = ProcessReturnRequest {
            action: "approve".to_string(),
            admin_notes: Some("Disetujui untuk penggantian".to_string()),
            refund_amount_override: None,
        };

        let processed = module.process_return(&ret.id, "admin", process_req).await.unwrap();
        assert_eq!(processed.status, "approved");

        // Update return status - Received
        let update_req = UpdateReturnStatusRequest {
            status: "received".to_string(),
            admin_notes: Some("Barang sudah sampai gudang".to_string()),
        };
        let updated = module.update_return_status(&ret.id, "admin", update_req).await.unwrap();
        assert_eq!(updated.status, "received");
    }
}
