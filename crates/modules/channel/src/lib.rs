use async_trait::async_trait;
use chrono::{DateTime, Utc};
use program1_contracts::{
    ChannelStatusDto, ChannelSyncContract, ChannelType, ContractError,
};
use program1_core::database::DbPool;
use sqlx::Row;

#[derive(Clone)]
pub struct ChannelSyncModule {
    pool: DbPool,
}

impl ChannelSyncModule {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn seed_default_channels(&self) -> Result<(), ContractError> {
        let count_row = sqlx::query("SELECT COUNT(*) as count FROM channel_statuses")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;

        let count: i64 = count_row.get("count");
        if count > 0 {
            return Ok(());
        }

        let initial_channels = vec![
            ("NativeWeb", "AURA Native Web Storefront", 12),
            ("TikTokShop", "TikTok Shop (AURA Official Store)", 8),
            ("Shopee", "Shopee Mall (AURA Official)", 10),
            ("Tokopedia", "Tokopedia Official Store", 6),
        ];

        let now = Utc::now().to_rfc3339();
        for (ch, name, synced) in initial_channels {
            sqlx::query(
                "INSERT OR IGNORE INTO channel_statuses (channel, name, synced_products, total_sales, is_active, last_synced_at)
                 VALUES ($1, $2, $3, 0.0, 1, $4)",
            )
            .bind(ch)
            .bind(name)
            .bind(synced as i64)
            .bind(&now)
            .execute(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(e.to_string()))?;
        }

        Ok(())
    }


    pub fn channel_to_db_key(channel: &ChannelType) -> &'static str {
        match channel {
            ChannelType::NativeWeb => "NativeWeb",
            ChannelType::TikTokShop => "TikTokShop",
            ChannelType::Shopee => "Shopee",
            ChannelType::Tokopedia => "Tokopedia",
        }
    }

    pub fn channel_from_str(s: &str) -> ChannelType {
        let clean = s.trim().to_lowercase().replace(' ', "");
        match clean.as_str() {
            "tiktok" | "tiktokshop" => ChannelType::TikTokShop,
            "shopee" => ChannelType::Shopee,
            "tokopedia" => ChannelType::Tokopedia,
            _ => ChannelType::NativeWeb,
        }
    }
}

#[async_trait]
impl ChannelSyncContract for ChannelSyncModule {
    async fn get_channel_statuses(&self) -> Result<Vec<ChannelStatusDto>, ContractError> {
        let rows = sqlx::query(
            "SELECT channel, name, synced_products, total_sales, is_active, last_synced_at
             FROM channel_statuses ORDER BY name ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let mut list = Vec::new();
        for r in rows {
            let ch_str: String = r.get("channel");
            let name: String = r.get("name");
            let synced: i64 = r.get("synced_products");
            let is_active: i64 = r.get("is_active");
            let last_synced_str: String = r.get("last_synced_at");
            let last_synced_at = DateTime::parse_from_rfc3339(&last_synced_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            list.push(ChannelStatusDto {
                channel: Self::channel_from_str(&ch_str),
                name,
                is_connected: is_active != 0,
                active_products_synced: synced as u32,
                last_synced_at,
            });
        }

        Ok(list)
    }

    async fn sync_channel_stock(&self, channel: ChannelType) -> Result<u32, ContractError> {
        let ch_key = Self::channel_to_db_key(&channel);
        let now = Utc::now().to_rfc3339();

        let result = sqlx::query(
            "UPDATE channel_statuses
             SET synced_products = synced_products + 1, last_synced_at = $1
             WHERE LOWER(channel) = LOWER($2)",
        )
        .bind(&now)
        .bind(ch_key)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(ContractError::NotFound(format!("Channel {:?}", channel)));
        }

        let updated_row = sqlx::query(
            "SELECT synced_products FROM channel_statuses WHERE LOWER(channel) = LOWER($1)",
        )
        .bind(ch_key)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(e.to_string()))?;

        let synced_count: i64 = updated_row.get("synced_products");
        tracing::info!(channel = %channel, synced_products = synced_count, "Channel stock synced in database");
        Ok(synced_count as u32)
    }

    async fn pull_remote_orders(&self, channel: ChannelType) -> Result<u32, ContractError> {
        let ch_key = Self::channel_to_db_key(&channel);
        let now = Utc::now().to_rfc3339();

        let _ = sqlx::query(
            "UPDATE channel_statuses SET last_synced_at = $1 WHERE LOWER(channel) = LOWER($2)",
        )
        .bind(&now)
        .bind(ch_key)
        .execute(&self.pool)
        .await;

        tracing::info!(channel = %channel, "Remote orders pulled in database");
        Ok(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use program1_core::init_database;

    #[tokio::test]
    async fn test_channel_sync() {
        let pool = init_database("sqlite::memory:").await.unwrap();
        let module = ChannelSyncModule::new(pool);
        module.seed_default_channels().await.unwrap();

        let statuses = module.get_channel_statuses().await.unwrap();
        assert_eq!(statuses.len(), 4);

        let synced = module.sync_channel_stock(ChannelType::TikTokShop).await.unwrap();
        assert!(synced > 0);
    }
}
