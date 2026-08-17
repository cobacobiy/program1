use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;

use program1_contracts::{
    ChannelStatusDto, ChannelSyncContract, ChannelType, ContractError,
};

#[derive(Clone)]
pub struct ChannelSyncModule {
    channels: Arc<RwLock<HashMap<ChannelType, ChannelStatusDto>>>,
}

impl ChannelSyncModule {
    pub fn new() -> Self {
        let initial_channels = vec![
            ChannelStatusDto {
                channel: ChannelType::NativeWeb,
                name: "AURA Native Web Storefront".to_string(),
                is_connected: true,
                active_products_synced: 12,
                last_synced_at: Utc::now(),
            },
            ChannelStatusDto {
                channel: ChannelType::TikTokShop,
                name: "TikTok Shop (AURA Official Store)".to_string(),
                is_connected: true,
                active_products_synced: 8,
                last_synced_at: Utc::now(),
            },
            ChannelStatusDto {
                channel: ChannelType::Shopee,
                name: "Shopee Mall (AURA Official)".to_string(),
                is_connected: true,
                active_products_synced: 10,
                last_synced_at: Utc::now(),
            },
            ChannelStatusDto {
                channel: ChannelType::Tokopedia,
                name: "Tokopedia Official Store".to_string(),
                is_connected: true,
                active_products_synced: 6,
                last_synced_at: Utc::now(),
            },
        ];

        let mut map = HashMap::new();
        for ch in initial_channels {
            map.insert(ch.channel.clone(), ch);
        }

        Self {
            channels: Arc::new(RwLock::new(map)),
        }
    }
}

impl Default for ChannelSyncModule {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChannelSyncContract for ChannelSyncModule {
    async fn get_channel_statuses(&self) -> Result<Vec<ChannelStatusDto>, ContractError> {
        let lock = self.channels.read().await;
        let mut list: Vec<ChannelStatusDto> = lock.values().cloned().collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(list)
    }

    async fn sync_channel_stock(&self, channel: ChannelType) -> Result<u32, ContractError> {
        let mut lock = self.channels.write().await;
        let status = lock
            .get_mut(&channel)
            .ok_or_else(|| ContractError::NotFound(format!("Channel {:?}", channel)))?;

        status.last_synced_at = Utc::now();
        status.active_products_synced += 1;
        let synced_count = status.active_products_synced;

        tracing::info!(channel = %channel, synced_products = synced_count, "Channel stock synced");
        Ok(synced_count)
    }

    async fn pull_remote_orders(&self, channel: ChannelType) -> Result<u32, ContractError> {
        let mut lock = self.channels.write().await;
        let status = lock
            .get_mut(&channel)
            .ok_or_else(|| ContractError::NotFound(format!("Channel {:?}", channel)))?;

        status.last_synced_at = Utc::now();
        tracing::info!(channel = %channel, "Remote orders pulled");
        Ok(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_channel_sync() {
        let module = ChannelSyncModule::new();
        let statuses = module.get_channel_statuses().await.unwrap();
        assert_eq!(statuses.len(), 4);

        let synced = module.sync_channel_stock(ChannelType::TikTokShop).await.unwrap();
        assert!(synced > 0);
    }
}
