use async_trait::async_trait;
use program1_contracts::{ContractError, NotificationCountDto, NotificationContract, NotificationDto};
use program1_core::database::DbPool;
use sqlx::Row;
use uuid::Uuid;

#[derive(Clone)]
pub struct NotificationModule {
    pool: DbPool,
}

impl NotificationModule {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NotificationContract for NotificationModule {
    async fn create_notification(
        &self,
        recipient_type: &str,
        recipient_id: &str,
        title: &str,
        message: &str,
        notification_type: &str,
        reference_id: Option<&str>,
        reference_type: Option<&str>,
    ) -> Result<NotificationDto, ContractError> {
        let id = Uuid::new_v4().to_string();

        let row = sqlx::query(
            r#"
            INSERT INTO notifications (
                id, recipient_type, recipient_id, title, message,
                notification_type, reference_id, reference_type, is_read, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, datetime('now'))
            RETURNING id, recipient_type, recipient_id, title, message,
                      notification_type, reference_id, reference_type, is_read, created_at
            "#,
        )
        .bind(&id)
        .bind(recipient_type)
        .bind(recipient_id)
        .bind(title)
        .bind(message)
        .bind(notification_type)
        .bind(reference_id)
        .bind(reference_type)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(format!("Failed to insert notification: {}", e)))?;

        let is_read_int: i64 = row.get("is_read");
        Ok(NotificationDto {
            id: row.get("id"),
            recipient_type: row.get("recipient_type"),
            recipient_id: row.get("recipient_id"),
            title: row.get("title"),
            message: row.get("message"),
            notification_type: row.get("notification_type"),
            reference_id: row.get("reference_id"),
            reference_type: row.get("reference_type"),
            is_read: is_read_int != 0,
            created_at: row.get("created_at"),
        })
    }

    async fn list_notifications(
        &self,
        recipient_type: &str,
        recipient_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<NotificationDto>, ContractError> {
        let rows = sqlx::query(
            r#"
            SELECT id, recipient_type, recipient_id, title, message,
                   notification_type, reference_id, reference_type, is_read, created_at
            FROM notifications
            WHERE recipient_type = $1 AND (recipient_id = $2 OR recipient_id = 'all')
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(recipient_type)
        .bind(recipient_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(format!("Failed to fetch notifications: {}", e)))?;

        let mut list = Vec::with_capacity(rows.len());
        for row in rows {
            let is_read_int: i64 = row.get("is_read");
            list.push(NotificationDto {
                id: row.get("id"),
                recipient_type: row.get("recipient_type"),
                recipient_id: row.get("recipient_id"),
                title: row.get("title"),
                message: row.get("message"),
                notification_type: row.get("notification_type"),
                reference_id: row.get("reference_id"),
                reference_type: row.get("reference_type"),
                is_read: is_read_int != 0,
                created_at: row.get("created_at"),
            });
        }

        Ok(list)
    }

    async fn get_unread_count(
        &self,
        recipient_type: &str,
        recipient_id: &str,
    ) -> Result<NotificationCountDto, ContractError> {
        let unread_row = sqlx::query(
            r#"
            SELECT COUNT(*) as unread_count
            FROM notifications
            WHERE recipient_type = $1 AND (recipient_id = $2 OR recipient_id = 'all') AND is_read = 0
            "#,
        )
        .bind(recipient_type)
        .bind(recipient_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(format!("Failed to count unread notifications: {}", e)))?;

        let total_row = sqlx::query(
            r#"
            SELECT COUNT(*) as total_count
            FROM notifications
            WHERE recipient_type = $1 AND (recipient_id = $2 OR recipient_id = 'all')
            "#,
        )
        .bind(recipient_type)
        .bind(recipient_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(format!("Failed to count total notifications: {}", e)))?;

        let unread: i64 = unread_row.get("unread_count");
        let total: i64 = total_row.get("total_count");

        Ok(NotificationCountDto { unread, total })
    }

    async fn mark_as_read(
        &self,
        notification_id: &str,
        recipient_id: &str,
    ) -> Result<(), ContractError> {
        let res = sqlx::query(
            r#"
            UPDATE notifications
            SET is_read = 1
            WHERE id = $1 AND (recipient_id = $2 OR recipient_id = 'all')
            "#,
        )
        .bind(notification_id)
        .bind(recipient_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(format!("Failed to mark notification as read: {}", e)))?;

        if res.rows_affected() == 0 {
            return Err(ContractError::NotFound(format!(
                "Notification {} not found or unauthorized",
                notification_id
            )));
        }

        Ok(())
    }

    async fn mark_all_as_read(
        &self,
        recipient_type: &str,
        recipient_id: &str,
    ) -> Result<(), ContractError> {
        sqlx::query(
            r#"
            UPDATE notifications
            SET is_read = 1
            WHERE recipient_type = $1 AND (recipient_id = $2 OR recipient_id = 'all') AND is_read = 0
            "#,
        )
        .bind(recipient_type)
        .bind(recipient_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(format!("Failed to mark all notifications as read: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use program1_core::init_database;

    #[tokio::test]
    async fn test_notification_lifecycle() {
        let pool = init_database("sqlite::memory:").await.unwrap();
        let module = NotificationModule::new(pool);

        // 1. Create 2 buyer notifications and 1 seller notification
        let notif1 = module
            .create_notification(
                "buyer",
                "buyer-123",
                "Pesanan Diproses",
                "Pesanan Anda sedang diproses oleh penjual",
                "order_status",
                Some("order-abc"),
                Some("order"),
            )
            .await
            .unwrap();

        assert_eq!(notif1.title, "Pesanan Diproses");
        assert_eq!(notif1.is_read, false);
        assert_eq!(notif1.reference_id.as_deref(), Some("order-abc"));

        let _notif2 = module
            .create_notification(
                "buyer",
                "buyer-123",
                "Pesanan Dikirim",
                "Pesanan Anda telah dikirim",
                "order_status",
                Some("order-abc"),
                Some("order"),
            )
            .await
            .unwrap();

        let _seller_notif = module
            .create_notification(
                "seller",
                "admin",
                "Pesanan Baru Masuk",
                "Pesanan #order-abc baru saja dibayar",
                "new_order",
                Some("order-abc"),
                Some("order"),
            )
            .await
            .unwrap();

        // 2. Check counts for buyer
        let buyer_counts = module.get_unread_count("buyer", "buyer-123").await.unwrap();
        assert_eq!(buyer_counts.unread, 2);
        assert_eq!(buyer_counts.total, 2);

        // Check counts for seller
        let seller_counts = module.get_unread_count("seller", "admin").await.unwrap();
        assert_eq!(seller_counts.unread, 1);
        assert_eq!(seller_counts.total, 1);

        // 3. List buyer notifications
        let buyer_list = module
            .list_notifications("buyer", "buyer-123", 10, 0)
            .await
            .unwrap();
        assert_eq!(buyer_list.len(), 2);

        // 4. Mark single as read
        module
            .mark_as_read(&notif1.id, "buyer-123")
            .await
            .unwrap();
        let updated_counts = module.get_unread_count("buyer", "buyer-123").await.unwrap();
        assert_eq!(updated_counts.unread, 1);
        assert_eq!(updated_counts.total, 2);

        // 5. Try to mark notification using wrong recipient ID -> Expect NotFound error
        let wrong_recipient_res = module.mark_as_read(&notif1.id, "buyer-999").await;
        assert!(wrong_recipient_res.is_err());

        // 6. Mark all as read
        module
            .mark_all_as_read("buyer", "buyer-123")
            .await
            .unwrap();
        let final_counts = module.get_unread_count("buyer", "buyer-123").await.unwrap();
        assert_eq!(final_counts.unread, 0);
        assert_eq!(final_counts.total, 2);
    }
}
