use std::sync::Arc;
use program1_contracts::{
    CatalogContract, CreateCatalogItemRequest, CreateUserAccountRequest,
    OrderContract, StorefrontOrderItemRequest, StorefrontOrderRequest, UserContract,
};

use program1_core::init_database;
use program1_module_catalog::CatalogModule;
use program1_module_inventory::InventoryModule;
use program1_module_order::OrderModule;
use program1_module_user::UserModule;

#[tokio::test]
async fn test_db_persistence_across_pool_reconnection() {
    let db_path = format!("sqlite:///tmp/program1_test_persist_{}.db?mode=rwc", uuid::Uuid::new_v4());

    // Phase 1: Initialize, create user, catalog item, and order
    {
        let pool1 = init_database(&db_path).await.expect("Failed to init db pool 1");
        let user1 = Arc::new(UserModule::new(pool1.clone()));
        let cat1 = Arc::new(CatalogModule::new(pool1.clone()));
        let inv1 = Arc::new(InventoryModule::new(pool1.clone(), cat1.clone()));
        let ord1 = Arc::new(OrderModule::new(pool1.clone(), cat1.clone(), inv1.clone()));

        let _ = user1.seed_default_users().await;
        let _ = cat1.seed_default_catalog().await;

        // Create new user
        let user = user1
            .create_account(CreateUserAccountRequest {
                username: "persistent_user".to_string(),
                full_name: "Persistent User".to_string(),
                role: "Staff".to_string(),
                accessible_menus: vec!["orders".to_string()],
            })
            .await
            .expect("Failed to create user");

        assert_eq!(user.username, "persistent_user");

        // Create new catalog product
        let product = cat1
            .create_item(CreateCatalogItemRequest {
                name: "Persistent Mechanical Switch Set".to_string(),
                sku: "SKU-SWITCH-PERSIST".to_string(),
                category: "Accessories".to_string(),
                price: 320000.0,
                stock: 50,
                image_url: None,
                description: Some("Durable linear switches".to_string()),
            })
            .await
            .expect("Failed to create catalog product");

        assert_eq!(product.sku, "SKU-SWITCH-PERSIST");

        // Create order
        let order = ord1
            .create_storefront_order(StorefrontOrderRequest {
                customer_name: "Persistent Buyer".to_string(),
                customer_email: "buyer@persist.com".to_string(),
                shipping_address: "Jakarta Barat".to_string(),
                items: vec![StorefrontOrderItemRequest {
                    product_id: product.id,
                    quantity: 3,
                }],
            })
            .await
            .expect("Failed to create order");

        assert_eq!(order.total_amount, 320000.0 * 3.0);
    }

    // Phase 2: Re-open database with a completely new connection pool and verify records exist!
    {
        let pool2 = init_database(&db_path).await.expect("Failed to init db pool 2");
        let user2 = Arc::new(UserModule::new(pool2.clone()));
        let cat2 = Arc::new(CatalogModule::new(pool2.clone()));
        let inv2 = Arc::new(InventoryModule::new(pool2.clone(), cat2.clone()));
        let ord2 = Arc::new(OrderModule::new(pool2.clone(), cat2.clone(), inv2.clone()));

        // Verify user persisted
        let accounts = user2.list_accounts().await.expect("Failed to list accounts");
        let found_user = accounts.iter().find(|a| a.username == "persistent_user");
        assert!(found_user.is_some(), "persistent_user should be found after reconnecting");

        // Verify catalog item persisted
        let items = cat2.list_items().await.expect("Failed to list catalog");
        let found_product = items.iter().find(|p| p.sku == "SKU-SWITCH-PERSIST");
        assert!(found_product.is_some(), "Product should persist across reconnecting");

        // Verify order persisted
        let orders = ord2.list_orders().await.expect("Failed to list orders");
        let found_order = orders.iter().find(|o| o.customer_name == "Persistent Buyer");
        assert!(found_order.is_some(), "Order should persist across reconnecting");
        assert_eq!(found_order.unwrap().items.len(), 1);
        assert_eq!(found_order.unwrap().items[0].quantity, 3);
    }

    // Cleanup test db
    let clean_path = db_path.trim_start_matches("sqlite://").split('?').next().unwrap_or("");
    let _ = std::fs::remove_file(clean_path);
}
