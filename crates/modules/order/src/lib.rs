use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;
use uuid::Uuid;

use program1_contracts::{
    ContractError, CreateOrderRequest, OrderContract, OrderDto, OrderItemDto,
    ProductContract, UserContract,
};

#[derive(Clone)]
pub struct OrderModule {
    store: Arc<RwLock<HashMap<Uuid, OrderDto>>>,
    user_contract: Arc<dyn UserContract>,
    product_contract: Arc<dyn ProductContract>,
}

impl OrderModule {
    pub fn new(
        user_contract: Arc<dyn UserContract>,
        product_contract: Arc<dyn ProductContract>,
    ) -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            user_contract,
            product_contract,
        }
    }
}

#[async_trait]
impl OrderContract for OrderModule {
    async fn get_order(&self, id: Uuid) -> Result<OrderDto, ContractError> {
        let lock = self.store.read().await;
        lock.get(&id)
            .cloned()
            .ok_or_else(|| ContractError::NotFound(format!("Order {}", id)))
    }

    async fn create_order(&self, req: CreateOrderRequest) -> Result<OrderDto, ContractError> {
        if req.items.is_empty() {
            return Err(ContractError::ValidationError("Order must contain at least one item".to_string()));
        }

        // 1. Verify User existence via UserContract interface
        let user = self.user_contract.get_user(req.user_id).await?;

        // 2. Validate products and reserve stock via ProductContract interface
        let mut order_items = Vec::new();
        let mut total_amount = 0.0;

        for item in &req.items {
            if item.quantity == 0 {
                return Err(ContractError::ValidationError("Item quantity must be greater than zero".to_string()));
            }

            let product = self.product_contract.get_product(item.product_id).await?;
            self.product_contract
                .reserve_stock(item.product_id, item.quantity)
                .await?;

            let item_total = product.price * (item.quantity as f64);
            total_amount += item_total;

            order_items.push(OrderItemDto {
                product_id: product.id,
                quantity: item.quantity,
                unit_price: product.price,
                total_price: item_total,
            });
        }

        let order = OrderDto {
            id: Uuid::new_v4(),
            user_id: user.id,
            user_name: user.name,
            items: order_items,
            total_amount,
            status: "COMPLETED".to_string(),
            created_at: Utc::now(),
        };

        let mut lock = self.store.write().await;
        lock.insert(order.id, order.clone());
        tracing::info!(order_id = %order.id, user_id = %order.user_id, total = %order.total_amount, "Order created successfully");
        Ok(order)
    }

    async fn list_orders(&self) -> Result<Vec<OrderDto>, ContractError> {
        let lock = self.store.read().await;
        let mut list: Vec<OrderDto> = lock.values().cloned().collect();
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use program1_module_product::ProductModule;
    use program1_module_user::UserModule;
    use program1_contracts::OrderItemRequest;

    #[tokio::test]
    async fn test_order_creation_contract_flow() {
        let user_module = Arc::new(UserModule::new());
        let product_module = Arc::new(ProductModule::new());
        let order_module = OrderModule::new(user_module.clone(), product_module.clone());

        let users = user_module.list_users().await.unwrap();
        let products = product_module.list_products().await.unwrap();

        let req = CreateOrderRequest {
            user_id: users[0].id,
            items: vec![OrderItemRequest {
                product_id: products[0].id,
                quantity: 2,
            }],
        };

        let order = order_module.create_order(req).await.unwrap();
        assert_eq!(order.user_id, users[0].id);
        assert_eq!(order.items.len(), 1);
        assert_eq!(order.items[0].quantity, 2);
        assert_eq!(order.status, "COMPLETED");

        let fetched = order_module.get_order(order.id).await.unwrap();
        assert_eq!(fetched.id, order.id);
    }
}
