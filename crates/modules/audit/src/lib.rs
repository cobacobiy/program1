use async_trait::async_trait;
use chrono::{DateTime, Utc};
use program1_contracts::{AuditContract, AuditLogEntry, ContractError};
use program1_core::database::DbPool;
use sqlx::Row;
use uuid::Uuid;

#[derive(Clone)]
pub struct AuditModule {
    pool: DbPool,
}

impl AuditModule {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    fn row_to_entry(row: &sqlx::sqlite::SqliteRow) -> Result<AuditLogEntry, ContractError> {
        let id_str: String = row.get("id");
        let id = Uuid::parse_str(&id_str)
            .map_err(|e| ContractError::Internal(format!("Corrupt UUID in audit log: {}", e)))?;

        let timestamp_str: String = row.get("timestamp");
        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let actor_id_str: Option<String> = row.get("actor_id");
        let actor_id = actor_id_str.and_then(|s| Uuid::parse_str(&s).ok());

        let resource_id_str: Option<String> = row.get("resource_id");
        let resource_id = resource_id_str.and_then(|s| Uuid::parse_str(&s).ok());

        Ok(AuditLogEntry {
            id,
            timestamp,
            actor_id,
            actor_username: row.get("actor_username"),
            action: row.get("action"),
            resource_type: row.get("resource_type"),
            resource_id,
            details: row.get("details"),
            ip_address: row.get("ip_address"),
        })
    }
}

#[async_trait]
impl AuditContract for AuditModule {
    async fn log_action(&self, entry: AuditLogEntry) -> Result<(), ContractError> {
        let actor_id_str = entry.actor_id.map(|id| id.to_string());
        let resource_id_str = entry.resource_id.map(|id| id.to_string());
        let timestamp_str = entry.timestamp.to_rfc3339();

        sqlx::query(
            "INSERT INTO audit_logs (id, timestamp, actor_id, actor_username, action, resource_type, resource_id, details, ip_address)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(entry.id.to_string())
        .bind(timestamp_str)
        .bind(actor_id_str)
        .bind(entry.actor_username)
        .bind(entry.action)
        .bind(entry.resource_type)
        .bind(resource_id_str)
        .bind(entry.details)
        .bind(entry.ip_address)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn get_logs(
        &self,
        resource_type: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AuditLogEntry>, ContractError> {
        let safe_limit = limit.clamp(1, 100) as i64;
        let safe_offset = offset as i64;

        let rows = if let Some(res) = resource_type {
            sqlx::query(
                "SELECT id, timestamp, actor_id, actor_username, action, resource_type, resource_id, details, ip_address
                 FROM audit_logs
                 WHERE LOWER(resource_type) = LOWER($1)
                 ORDER BY timestamp DESC
                 LIMIT $2 OFFSET $3",
            )
            .bind(res)
            .bind(safe_limit)
            .bind(safe_offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?
        } else {
            sqlx::query(
                "SELECT id, timestamp, actor_id, actor_username, action, resource_type, resource_id, details, ip_address
                 FROM audit_logs
                 ORDER BY timestamp DESC
                 LIMIT $1 OFFSET $2",
            )
            .bind(safe_limit)
            .bind(safe_offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?
        };

        let mut entries = Vec::new();
        for r in &rows {
            entries.push(Self::row_to_entry(r)?);
        }

        Ok(entries)
    }

    async fn get_logs_by_actor(&self, actor_id: Uuid) -> Result<Vec<AuditLogEntry>, ContractError> {
        let rows = sqlx::query(
            "SELECT id, timestamp, actor_id, actor_username, action, resource_type, resource_id, details, ip_address
             FROM audit_logs
             WHERE actor_id = $1
             ORDER BY timestamp DESC
             LIMIT 100",
        )
        .bind(actor_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let mut entries = Vec::new();
        for r in &rows {
            entries.push(Self::row_to_entry(r)?);
        }

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use program1_core::init_database;

    #[tokio::test]
    async fn test_audit_log_creation_and_queries() {
        let pool = init_database("sqlite::memory:").await.unwrap();
        let module = AuditModule::new(pool);

        let user_id = Uuid::new_v4();
        let entry1 = AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor_id: Some(user_id),
            actor_username: "admin".to_string(),
            action: "LOGIN_SUCCESS".to_string(),
            resource_type: "user".to_string(),
            resource_id: Some(user_id),
            details: r#"{"role":"Super Admin"}"#.to_string(),
            ip_address: Some("127.0.0.1".to_string()),
        };

        let entry2 = AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor_id: None,
            actor_username: "anonymous".to_string(),
            action: "ORDER_CREATED".to_string(),
            resource_type: "order".to_string(),
            resource_id: Some(Uuid::new_v4()),
            details: r#"{"total":150000.0}"#.to_string(),
            ip_address: Some("192.168.1.50".to_string()),
        };

        module.log_action(entry1).await.unwrap();
        module.log_action(entry2).await.unwrap();

        let all_logs = module.get_logs(None, 10, 0).await.unwrap();
        assert_eq!(all_logs.len(), 2);

        let order_logs = module.get_logs(Some("order"), 10, 0).await.unwrap();
        assert_eq!(order_logs.len(), 1);
        assert_eq!(order_logs[0].action, "ORDER_CREATED");

        let actor_logs = module.get_logs_by_actor(user_id).await.unwrap();
        assert_eq!(actor_logs.len(), 1);
        assert_eq!(actor_logs[0].actor_username, "admin");
    }
}
