use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;

use program1_contracts::{
    AnalyticsContract, CatalogContract, ChannelRevenueDto, ChannelType, ContractError,
    OrderContract, SalesAnalyticsDto,
};

#[derive(Clone)]
pub struct AnalyticsModule {
    catalog_contract: Arc<dyn CatalogContract>,
    order_contract: Arc<dyn OrderContract>,
}

impl AnalyticsModule {
    pub fn new(
        catalog_contract: Arc<dyn CatalogContract>,
        order_contract: Arc<dyn OrderContract>,
    ) -> Self {
        Self {
            catalog_contract,
            order_contract,
        }
    }
}

#[async_trait]
impl AnalyticsContract for AnalyticsModule {
    async fn get_sales_analytics(&self) -> Result<SalesAnalyticsDto, ContractError> {
        let products = self.catalog_contract.list_items().await?;
        let orders = self.order_contract.list_orders().await?;

        let mut gross_revenue = 0.0;
        let mut channel_revenue_map: HashMap<ChannelType, (u32, f64)> = HashMap::new();

        // Seed initial channels
        channel_revenue_map.insert(ChannelType::NativeWeb, (0, 0.0));
        channel_revenue_map.insert(ChannelType::TikTokShop, (0, 0.0));
        channel_revenue_map.insert(ChannelType::Shopee, (0, 0.0));
        channel_revenue_map.insert(ChannelType::Tokopedia, (0, 0.0));

        for order in &orders {
            gross_revenue += order.total_amount;
            let entry = channel_revenue_map.entry(order.channel.clone()).or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += order.total_amount;
        }

        let channel_breakdown = vec![
            ChannelRevenueDto {
                channel: ChannelType::NativeWeb,
                channel_name: ChannelType::NativeWeb.to_string(),
                total_orders: channel_revenue_map.get(&ChannelType::NativeWeb).map(|x| x.0).unwrap_or(0),
                total_revenue: channel_revenue_map.get(&ChannelType::NativeWeb).map(|x| x.1).unwrap_or(0.0),
            },
            ChannelRevenueDto {
                channel: ChannelType::TikTokShop,
                channel_name: ChannelType::TikTokShop.to_string(),
                total_orders: channel_revenue_map.get(&ChannelType::TikTokShop).map(|x| x.0).unwrap_or(0),
                total_revenue: channel_revenue_map.get(&ChannelType::TikTokShop).map(|x| x.1).unwrap_or(0.0),
            },
            ChannelRevenueDto {
                channel: ChannelType::Shopee,
                channel_name: ChannelType::Shopee.to_string(),
                total_orders: channel_revenue_map.get(&ChannelType::Shopee).map(|x| x.0).unwrap_or(0),
                total_revenue: channel_revenue_map.get(&ChannelType::Shopee).map(|x| x.1).unwrap_or(0.0),
            },
            ChannelRevenueDto {
                channel: ChannelType::Tokopedia,
                channel_name: ChannelType::Tokopedia.to_string(),
                total_orders: channel_revenue_map.get(&ChannelType::Tokopedia).map(|x| x.0).unwrap_or(0),
                total_revenue: channel_revenue_map.get(&ChannelType::Tokopedia).map(|x| x.1).unwrap_or(0.0),
            },
        ];

        Ok(SalesAnalyticsDto {
            gross_revenue,
            total_orders: orders.len() as u32,
            active_products: products.len() as u32,
            channel_breakdown,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use program1_core::init_database;
    use program1_module_catalog::CatalogModule;
    use program1_module_inventory::InventoryModule;
    use program1_module_order::OrderModule;

    #[tokio::test]
    async fn test_analytics() {
        let pool = init_database("sqlite::memory:").await.unwrap();
        let catalog = Arc::new(CatalogModule::new(pool.clone()));
        catalog.seed_default_catalog().await.unwrap();
        let inventory = Arc::new(InventoryModule::new(pool.clone(), catalog.clone()));
        let order = Arc::new(OrderModule::new(pool, catalog.clone(), inventory.clone()));
        let analytics = AnalyticsModule::new(catalog.clone(), order.clone());

        let stats = analytics.get_sales_analytics().await.unwrap();
        assert!(stats.active_products >= 3);
    }
}
