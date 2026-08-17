use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;
use uuid::Uuid;

use program1_contracts::{
    CatalogContract, ChannelType, ContractError, InventoryContract, OmniOrderDto,
    OrderContract, OrderItemDto, StorefrontOrderItemRequest, StorefrontOrderRequest,
};

#[derive(Clone)]
pub struct OrderModule {
    store: Arc<RwLock<HashMap<Uuid, OmniOrderDto>>>,
    catalog_contract: Arc<dyn CatalogContract>,
    inventory_contract: Arc<dyn InventoryContract>,
}

impl OrderModule {
    pub fn new(
        catalog_contract: Arc<dyn CatalogContract>,
        inventory_contract: Arc<dyn InventoryContract>,
    ) -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            catalog_contract,
            inventory_contract,
        }
    }
}

#[async_trait]
impl OrderContract for OrderModule {
    async fn get_order(&self, id: Uuid) -> Result<OmniOrderDto, ContractError> {
        let lock = self.store.read().await;
        lock.get(&id)
            .cloned()
            .ok_or_else(|| ContractError::NotFound(format!("Order {}", id)))
    }

    async fn create_storefront_order(&self, req: StorefrontOrderRequest) -> Result<OmniOrderDto, ContractError> {
        if req.items.is_empty() {
            return Err(ContractError::ValidationError("Order must contain at least one item".to_string()));
        }
        if req.customer_name.trim().is_empty() {
            return Err(ContractError::ValidationError("Customer name is required".to_string()));
        }

        let mut order_items = Vec::new();
        let mut total_amount = 0.0;

        for item in &req.items {
            if item.quantity == 0 {
                return Err(ContractError::ValidationError("Item quantity must be > 0".to_string()));
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

        let order = OmniOrderDto {
            id: Uuid::new_v4(),
            channel: ChannelType::NativeWeb,
            customer_name: req.customer_name.trim().to_string(),
            customer_email: req.customer_email.trim().to_string(),
            shipping_address: req.shipping_address.trim().to_string(),
            items: order_items,
            total_amount,
            status: "PAID".to_string(),
            created_at: Utc::now(),
        };

        let mut lock = self.store.write().await;
        lock.insert(order.id, order.clone());
        tracing::info!(order_id = %order.id, channel = %order.channel, total = %order.total_amount, "Storefront Order Created");
        Ok(order)
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

        let order = OmniOrderDto {
            id: Uuid::new_v4(),
            channel,
            customer_name,
            customer_email: "marketplace_buyer@auto.com".to_string(),
            shipping_address: "Marketplace Logistics Address".to_string(),
            items: order_items,
            total_amount,
            status: "PROCESSING".to_string(),
            created_at: Utc::now(),
        };

        let mut lock = self.store.write().await;
        lock.insert(order.id, order.clone());
        Ok(order)
    }

    async fn list_orders(&self) -> Result<Vec<OmniOrderDto>, ContractError> {
        let lock = self.store.read().await;
        let mut list: Vec<OmniOrderDto> = lock.values().cloned().collect();
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use program1_module_catalog::CatalogModule;
    use program1_module_inventory::InventoryModule;

    #[tokio::test]
    async fn test_order_creation_contract() {
        let catalog = Arc::new(CatalogModule::new());
        let inventory = Arc::new(InventoryModule::new(catalog.clone()));
        let order_module = OrderModule::new(catalog.clone(), inventory.clone());

        let products = catalog.list_items().await.unwrap();
        let req = StorefrontOrderRequest {
            customer_name: "Customer Jane".to_string(),
            customer_email: "jane@test.com".to_string(),
            shipping_address: "Jakarta Selatan".to_string(),
            items: vec![StorefrontOrderItemRequest {
                product_id: products[0].id,
                quantity: 2,
            }],
        };

        let order = order_module.create_storefront_order(req).await.unwrap();
        assert_eq!(order.customer_name, "Customer Jane");
        assert_eq!(order.channel, ChannelType::NativeWeb);
        assert_eq!(order.items.len(), 1);
    }
}
