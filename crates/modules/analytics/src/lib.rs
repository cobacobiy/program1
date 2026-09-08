use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use std::collections::HashMap;
use std::sync::Arc;

use program1_contracts::{
    AnalyticsContract, CatalogContract, ChannelRevenueDto, ChannelType, ContractError,
    OrderContract, SalesAnalyticsDto, SalesReportItem, SalesReportQuery, SalesReportResponse,
    SalesReportRow, SalesReportSummary,
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
            let entry = channel_revenue_map
                .entry(order.channel.clone())
                .or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += order.total_amount;
        }

        let channel_breakdown = vec![
            ChannelRevenueDto {
                channel: ChannelType::NativeWeb,
                channel_name: ChannelType::NativeWeb.to_string(),
                total_orders: channel_revenue_map
                    .get(&ChannelType::NativeWeb)
                    .map(|x| x.0)
                    .unwrap_or(0),
                total_revenue: channel_revenue_map
                    .get(&ChannelType::NativeWeb)
                    .map(|x| x.1)
                    .unwrap_or(0.0),
            },
            ChannelRevenueDto {
                channel: ChannelType::TikTokShop,
                channel_name: ChannelType::TikTokShop.to_string(),
                total_orders: channel_revenue_map
                    .get(&ChannelType::TikTokShop)
                    .map(|x| x.0)
                    .unwrap_or(0),
                total_revenue: channel_revenue_map
                    .get(&ChannelType::TikTokShop)
                    .map(|x| x.1)
                    .unwrap_or(0.0),
            },
            ChannelRevenueDto {
                channel: ChannelType::Shopee,
                channel_name: ChannelType::Shopee.to_string(),
                total_orders: channel_revenue_map
                    .get(&ChannelType::Shopee)
                    .map(|x| x.0)
                    .unwrap_or(0),
                total_revenue: channel_revenue_map
                    .get(&ChannelType::Shopee)
                    .map(|x| x.1)
                    .unwrap_or(0.0),
            },
            ChannelRevenueDto {
                channel: ChannelType::Tokopedia,
                channel_name: ChannelType::Tokopedia.to_string(),
                total_orders: channel_revenue_map
                    .get(&ChannelType::Tokopedia)
                    .map(|x| x.0)
                    .unwrap_or(0),
                total_revenue: channel_revenue_map
                    .get(&ChannelType::Tokopedia)
                    .map(|x| x.1)
                    .unwrap_or(0.0),
            },
        ];

        Ok(SalesAnalyticsDto {
            gross_revenue,
            total_orders: orders.len() as u32,
            active_products: products.len() as u32,
            channel_breakdown,
        })
    }

    async fn generate_sales_report(
        &self,
        query: SalesReportQuery,
    ) -> Result<SalesReportResponse, ContractError> {
        let orders = self.order_contract.list_orders().await?;

        // Parse optional date_from (start of day: 00:00:00 UTC)
        let date_from_opt: Option<DateTime<Utc>> = query
            .date_from
            .as_deref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .and_then(|s| {
                NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .ok()
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                    .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
            });

        // Parse optional date_to (end of day: 23:59:59 UTC)
        let date_to_opt: Option<DateTime<Utc>> = query
            .date_to
            .as_deref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .and_then(|s| {
                NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .ok()
                    .and_then(|d| d.and_hms_opt(23, 59, 59))
                    .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
            });

        let status_filter_clean = query
            .status_filter
            .as_deref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty() && s != "all" && s != "semua");

        let mut rows = Vec::new();
        let mut total_revenue_cents: i64 = 0;
        let mut total_items_sold: i32 = 0;
        let mut orders_by_status: HashMap<String, i32> = HashMap::new();

        for order in orders {
            // Check date range
            if let Some(df) = date_from_opt {
                if order.created_at < df {
                    continue;
                }
            }
            if let Some(dt) = date_to_opt {
                if order.created_at > dt {
                    continue;
                }
            }

            // Check status filter
            let status_lower = order.status.to_lowercase();
            if let Some(ref sf) = status_filter_clean {
                if &status_lower != sf {
                    continue;
                }
            }

            // Status aggregation
            *orders_by_status.entry(order.status.clone()).or_insert(0) += 1;

            // Compute items
            let mut row_items = Vec::new();
            let mut subtotal_cents: i64 = 0;
            let mut order_item_count: i32 = 0;

            for item in &order.items {
                let unit_price_cents = (item.unit_price * 100.0).round() as i64;
                let item_qty = item.quantity as i32;
                subtotal_cents += (item.total_price * 100.0).round() as i64;
                order_item_count += item_qty;

                row_items.push(SalesReportItem {
                    product_name: item.product_name.clone(),
                    quantity: item_qty,
                    unit_price_cents,
                });
            }

            let shipping_cents = order.shipping_cost_cents;
            let total_cents = (order.total_amount * 100.0).round() as i64;
            // discount = subtotal + shipping - total
            let discount_cents = (subtotal_cents + shipping_cents - total_cents).max(0);

            total_revenue_cents += total_cents;
            total_items_sold += order_item_count;

            // Payment status derivation
            let payment_status = match status_lower.as_str() {
                "paid" | "processing" | "shipped" | "delivered" | "completed" => "paid".to_string(),
                "cancelled" => "cancelled".to_string(),
                _ => "pending".to_string(),
            };

            rows.push(SalesReportRow {
                order_id: order.id.to_string(),
                order_date: order.created_at.to_rfc3339(),
                buyer_name: order.customer_name,
                buyer_email: if order.customer_email.is_empty() {
                    None
                } else {
                    Some(order.customer_email)
                },
                items: row_items,
                subtotal_cents,
                shipping_cents,
                discount_cents,
                total_cents,
                status: order.status,
                payment_status,
            });
        }

        let total_orders = rows.len() as i32;
        let average_order_value_cents = if total_orders > 0 {
            total_revenue_cents / total_orders as i64
        } else {
            0
        };

        let summary = SalesReportSummary {
            total_revenue_cents,
            total_orders,
            average_order_value_cents,
            total_items_sold,
            orders_by_status,
        };

        Ok(SalesReportResponse { summary, rows })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use program1_contracts::StorefrontOrderItemRequest;
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

    #[tokio::test]
    async fn test_generate_sales_report_empty_and_filters() {
        let pool = init_database("sqlite::memory:").await.unwrap();
        let catalog = Arc::new(CatalogModule::new(pool.clone()));
        catalog.seed_default_catalog().await.unwrap();
        let inventory = Arc::new(InventoryModule::new(pool.clone(), catalog.clone()));
        let order = Arc::new(OrderModule::new(pool.clone(), catalog.clone(), inventory.clone()));
        let analytics = AnalyticsModule::new(catalog.clone(), order.clone());

        // Empty report test
        let query = SalesReportQuery {
            date_from: None,
            date_to: None,
            status_filter: None,
        };
        let res = analytics.generate_sales_report(query).await.unwrap();
        assert_eq!(res.summary.total_orders, 0);
        assert_eq!(res.summary.total_revenue_cents, 0);
        assert_eq!(res.rows.len(), 0);

        // Seed 1 order via marketplace order
        let items = catalog.list_items().await.unwrap();
        let item_req = vec![StorefrontOrderItemRequest {
            product_id: items[0].id,
            quantity: 2,
        }];
        let created = order
            .create_marketplace_order(
                ChannelType::NativeWeb,
                "Test Buyer".to_string(),
                item_req,
            )
            .await
            .unwrap();

        // Query all
        let all_res = analytics
            .generate_sales_report(SalesReportQuery::default())
            .await
            .unwrap();
        assert_eq!(all_res.summary.total_orders, 1);
        assert_eq!(all_res.summary.total_items_sold, 2);
        assert_eq!(all_res.rows[0].order_id, created.id.to_string());
        assert_eq!(all_res.rows[0].buyer_name, "Test Buyer");

        // Filter by matching status
        let status_res = analytics
            .generate_sales_report(SalesReportQuery {
                date_from: None,
                date_to: None,
                status_filter: Some("processing".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(status_res.summary.total_orders, 1);

        // Filter by non-matching status
        let no_status_res = analytics
            .generate_sales_report(SalesReportQuery {
                date_from: None,
                date_to: None,
                status_filter: Some("delivered".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(no_status_res.summary.total_orders, 0);

        // Filter by past date (out of range)
        let past_res = analytics
            .generate_sales_report(SalesReportQuery {
                date_from: Some("2020-01-01".to_string()),
                date_to: Some("2020-01-02".to_string()),
                status_filter: None,
            })
            .await
            .unwrap();
        assert_eq!(past_res.summary.total_orders, 0);
    }
}
