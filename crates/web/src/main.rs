use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use uuid::Uuid;

use program1_contracts::{
    AnalyticsContract, CatalogContract, ChannelSyncContract, ChannelType, ContractError,
    CreateCatalogItemRequest, InventoryContract, OrderContract, StorefrontOrderRequest,
};
use program1_core::init_tracing;
use program1_module_analytics::AnalyticsModule;
use program1_module_catalog::CatalogModule;
use program1_module_channel::ChannelSyncModule;
use program1_module_inventory::InventoryModule;
use program1_module_order::OrderModule;

#[derive(Clone)]
pub struct AppState {
    pub store_name: String,
    pub store_currency: String,
    pub catalog_contract: Arc<dyn CatalogContract>,
    pub inventory_contract: Arc<dyn InventoryContract>,
    pub channel_contract: Arc<dyn ChannelSyncContract>,
    pub order_contract: Arc<dyn OrderContract>,
    pub analytics_contract: Arc<dyn AnalyticsContract>,
}

#[tokio::main]
async fn main() {
    init_tracing();

    let store_name = env::var("STORE_NAME").unwrap_or_else(|_| "AURA Storefront".to_string());
    let store_currency = env::var("STORE_CURRENCY").unwrap_or_else(|_| "IDR".to_string());

    // 1. Instantiate domain modules
    let catalog_module = Arc::new(CatalogModule::new());
    let inventory_module = Arc::new(InventoryModule::new(catalog_module.clone()));
    let channel_module = Arc::new(ChannelSyncModule::new());
    let order_module = Arc::new(OrderModule::new(
        catalog_module.clone(),
        inventory_module.clone(),
    ));
    let analytics_module = Arc::new(AnalyticsModule::new(
        catalog_module.clone(),
        order_module.clone(),
    ));

    let state = AppState {
        store_name,
        store_currency,
        catalog_contract: catalog_module,
        inventory_contract: inventory_module,
        channel_contract: channel_module,
        order_contract: order_module,
        analytics_contract: analytics_module,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_routes = Router::new()
        // Health & Store Info
        .route("/health", get(health_check))
        .route("/api/v1/store/info", get(get_store_info))
        // Catalog
        .route("/api/v1/catalog", get(list_catalog).post(create_catalog_item))
        .route("/api/v1/catalog/:id", get(get_catalog_item))
        // Inventory
        .route("/api/v1/inventory/:id", get(get_inventory_stock))
        // Channels
        .route("/api/v1/channels", get(list_channels))
        .route("/api/v1/channels/sync/:channel", post(sync_channel))
        // Orders
        .route("/api/v1/orders", get(list_orders).post(create_storefront_order))
        .route("/api/v1/orders/:id", get(get_order))
        .route("/api/v1/orders/marketplace", post(create_marketplace_order))
        // Analytics
        .route("/api/v1/analytics", get(get_analytics))
        .with_state(state);

    // Static pages
    let admin_page = std::fs::read_to_string("crates/web/static/index.html")
        .unwrap_or_else(|_| "<h1>Admin Hub</h1>".to_string());
    let store_page = std::fs::read_to_string("crates/web/static/store.html")
        .unwrap_or_else(|_| "<h1>Storefront</h1>".to_string());

    let store_page_alt = store_page.clone();

    let app = Router::new()
        .route("/admin", get(move || async move { Html(admin_page) }))
        .route("/store", get(move || async move { Html(store_page_alt) }))
        .route("/", get(move || async move { Html(store_page) }))
        .merge(api_routes)
        .nest_service("/assets", ServeDir::new("crates/web/static"))
        .layer(cors);

    let port: u16 = env::var("APP_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .unwrap_or(8080);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Starting Program1 Omnichannel Engine on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// --- HANDLERS ---

async fn health_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "status": "healthy",
            "app": "program1",
            "architecture": "modular_monolith",
            "version": "0.1.0"
        })),
    )
}

async fn get_store_info(State(state): State<AppState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "store_name": state.store_name,
            "currency": state.store_currency,
        })),
    )
}

fn map_contract_error(err: ContractError) -> (StatusCode, Json<serde_json::Value>) {
    match err {
        ContractError::NotFound(msg) => (StatusCode::NOT_FOUND, Json(json!({ "error": msg }))),
        ContractError::ValidationError(msg) => (StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))),
        ContractError::InsufficientStock { product_id, requested, available } => (
            StatusCode::CONFLICT,
            Json(json!({
                "error": "Insufficient stock",
                "product_id": product_id,
                "requested": requested,
                "available": available
            })),
        ),
        ContractError::ChannelSyncError(msg) => (StatusCode::BAD_GATEWAY, Json(json!({ "error": msg }))),
        ContractError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": msg }))),
    }
}

// Catalog Handlers
async fn list_catalog(State(state): State<AppState>) -> impl IntoResponse {
    match state.catalog_contract.list_items().await {
        Ok(items) => (StatusCode::OK, Json(json!(items))),
        Err(e) => map_contract_error(e),
    }
}

async fn get_catalog_item(Path(id): Path<Uuid>, State(state): State<AppState>) -> impl IntoResponse {
    match state.catalog_contract.get_item(id).await {
        Ok(item) => (StatusCode::OK, Json(json!(item))),
        Err(e) => map_contract_error(e),
    }
}

async fn create_catalog_item(
    State(state): State<AppState>,
    Json(payload): Json<CreateCatalogItemRequest>,
) -> impl IntoResponse {
    match state.catalog_contract.create_item(payload).await {
        Ok(item) => (StatusCode::CREATED, Json(json!(item))),
        Err(e) => map_contract_error(e),
    }
}

// Inventory Handlers
async fn get_inventory_stock(Path(id): Path<Uuid>, State(state): State<AppState>) -> impl IntoResponse {
    match state.inventory_contract.get_stock(id).await {
        Ok(stock) => (StatusCode::OK, Json(json!(stock))),
        Err(e) => map_contract_error(e),
    }
}

// Channel Sync Handlers
async fn list_channels(State(state): State<AppState>) -> impl IntoResponse {
    match state.channel_contract.get_channel_statuses().await {
        Ok(channels) => (StatusCode::OK, Json(json!(channels))),
        Err(e) => map_contract_error(e),
    }
}

async fn sync_channel(Path(channel_name): Path<String>, State(state): State<AppState>) -> impl IntoResponse {
    let channel = match channel_name.to_lowercase().as_str() {
        "tiktok" | "tiktokshop" => ChannelType::TikTokShop,
        "shopee" => ChannelType::Shopee,
        "tokopedia" => ChannelType::Tokopedia,
        _ => ChannelType::NativeWeb,
    };

    match state.channel_contract.sync_channel_stock(channel).await {
        Ok(count) => (StatusCode::OK, Json(json!({ "synced_products": count }))),
        Err(e) => map_contract_error(e),
    }
}

// Order Handlers
async fn list_orders(State(state): State<AppState>) -> impl IntoResponse {
    match state.order_contract.list_orders().await {
        Ok(orders) => (StatusCode::OK, Json(json!(orders))),
        Err(e) => map_contract_error(e),
    }
}

async fn get_order(Path(id): Path<Uuid>, State(state): State<AppState>) -> impl IntoResponse {
    match state.order_contract.get_order(id).await {
        Ok(order) => (StatusCode::OK, Json(json!(order))),
        Err(e) => map_contract_error(e),
    }
}

async fn create_storefront_order(
    State(state): State<AppState>,
    Json(payload): Json<StorefrontOrderRequest>,
) -> impl IntoResponse {
    match state.order_contract.create_storefront_order(payload).await {
        Ok(order) => (StatusCode::CREATED, Json(json!(order))),
        Err(e) => map_contract_error(e),
    }
}

#[derive(serde::Deserialize)]
struct MarketplaceOrderReq {
    channel: String,
    customer_name: String,
    items: Vec<program1_contracts::StorefrontOrderItemRequest>,
}

async fn create_marketplace_order(
    State(state): State<AppState>,
    Json(payload): Json<MarketplaceOrderReq>,
) -> impl IntoResponse {
    let channel = match payload.channel.to_lowercase().as_str() {
        "tiktok" | "tiktokshop" => ChannelType::TikTokShop,
        "shopee" => ChannelType::Shopee,
        "tokopedia" => ChannelType::Tokopedia,
        _ => ChannelType::NativeWeb,
    };

    match state.order_contract.create_marketplace_order(channel, payload.customer_name, payload.items).await {
        Ok(order) => (StatusCode::CREATED, Json(json!(order))),
        Err(e) => map_contract_error(e),
    }
}

// Analytics Handlers
async fn get_analytics(State(state): State<AppState>) -> impl IntoResponse {
    match state.analytics_contract.get_sales_analytics().await {
        Ok(analytics) => (StatusCode::OK, Json(json!(analytics))),
        Err(e) => map_contract_error(e),
    }
}
