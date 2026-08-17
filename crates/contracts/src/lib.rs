use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug, Serialize, Deserialize, Clone)]
pub enum ContractError {
    #[error("Resource not found: {0}")]
    NotFound(String),
    #[error("Validation failed: {0}")]
    ValidationError(String),
    #[error("Insufficient stock for product {product_id}: requested {requested}, available {available}")]
    InsufficientStock {
        product_id: Uuid,
        requested: u32,
        available: u32,
    },
    #[error("Channel sync failed: {0}")]
    ChannelSyncError(String),
    #[error("Internal module error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ChannelType {
    NativeWeb,
    TikTokShop,
    Shopee,
    Tokopedia,
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelType::NativeWeb => write!(f, "Native Web Storefront"),
            ChannelType::TikTokShop => write!(f, "TikTok Shop"),
            ChannelType::Shopee => write!(f, "Shopee Marketplace"),
            ChannelType::Tokopedia => write!(f, "Tokopedia"),
        }
    }
}

// --- CATALOG CONTRACT ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogItemDto {
    pub id: Uuid,
    pub name: String,
    pub sku: String,
    pub category: String,
    pub price: f64,
    pub stock: u32,
    pub image_url: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCatalogItemRequest {
    pub name: String,
    pub sku: String,
    pub category: String,
    pub price: f64,
    pub stock: u32,
    pub image_url: Option<String>,
    pub description: Option<String>,
}

#[async_trait]
pub trait CatalogContract: Send + Sync {
    async fn list_items(&self) -> Result<Vec<CatalogItemDto>, ContractError>;
    async fn get_item(&self, id: Uuid) -> Result<CatalogItemDto, ContractError>;
    async fn create_item(&self, req: CreateCatalogItemRequest) -> Result<CatalogItemDto, ContractError>;
}

// --- INVENTORY CONTRACT (Ginee OMS Multi-Stock) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryStockDto {
    pub product_id: Uuid,
    pub sku: String,
    pub product_name: String,
    pub image_url: String,
    pub average_purchase_price: f64,
    pub warehouse_stock: u32,
    pub spare_stock: u32,
    pub locked_stock: u32,
    pub promotion_stock: u32,
    pub safety_stock: u32,
    pub available_stock: u32,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyStockLogDto {
    pub id: Uuid,
    pub product_id: Uuid,
    pub old_safety_stock: u32,
    pub new_safety_stock: u32,
    pub admin_note: String,
    pub updated_by: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSafetyStockRequest {
    pub new_safety_stock: u32,
    pub admin_note: String,
    pub updated_by: Option<String>,
}

#[async_trait]
pub trait InventoryContract: Send + Sync {
    async fn get_all_stocks(&self) -> Result<Vec<InventoryStockDto>, ContractError>;
    async fn get_stock(&self, product_id: Uuid) -> Result<InventoryStockDto, ContractError>;
    async fn reserve_stock(&self, product_id: Uuid, quantity: u32) -> Result<(), ContractError>;
    async fn update_safety_stock(
        &self,
        product_id: Uuid,
        new_safety_stock: u32,
        admin_note: String,
        updated_by: String,
    ) -> Result<InventoryStockDto, ContractError>;
    async fn get_safety_stock_logs(&self, product_id: Uuid) -> Result<Vec<SafetyStockLogDto>, ContractError>;
}

// --- CHANNEL SYNC CONTRACT ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStatusDto {
    pub channel: ChannelType,
    pub name: String,
    pub is_connected: bool,
    pub active_products_synced: u32,
    pub last_synced_at: DateTime<Utc>,
}

#[async_trait]
pub trait ChannelSyncContract: Send + Sync {
    async fn get_channel_statuses(&self) -> Result<Vec<ChannelStatusDto>, ContractError>;
    async fn sync_channel_stock(&self, channel: ChannelType) -> Result<u32, ContractError>;
    async fn pull_remote_orders(&self, channel: ChannelType) -> Result<u32, ContractError>;
}

// --- ORDER CONTRACT ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItemDto {
    pub product_id: Uuid,
    pub product_name: String,
    pub quantity: u32,
    pub unit_price: f64,
    pub total_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniOrderDto {
    pub id: Uuid,
    pub channel: ChannelType,
    pub customer_name: String,
    pub customer_email: String,
    pub shipping_address: String,
    pub items: Vec<OrderItemDto>,
    pub total_amount: f64,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorefrontOrderItemRequest {
    pub product_id: Uuid,
    pub quantity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorefrontOrderRequest {
    pub customer_name: String,
    pub customer_email: String,
    pub shipping_address: String,
    pub items: Vec<StorefrontOrderItemRequest>,
}

#[async_trait]
pub trait OrderContract: Send + Sync {
    async fn create_storefront_order(&self, req: StorefrontOrderRequest) -> Result<OmniOrderDto, ContractError>;
    async fn create_marketplace_order(&self, channel: ChannelType, customer_name: String, items: Vec<StorefrontOrderItemRequest>) -> Result<OmniOrderDto, ContractError>;
    async fn list_orders(&self) -> Result<Vec<OmniOrderDto>, ContractError>;
    async fn get_order(&self, id: Uuid) -> Result<OmniOrderDto, ContractError>;
}

// --- ANALYTICS CONTRACT ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelRevenueDto {
    pub channel: ChannelType,
    pub channel_name: String,
    pub total_orders: u32,
    pub total_revenue: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesAnalyticsDto {
    pub gross_revenue: f64,
    pub total_orders: u32,
    pub active_products: u32,
    pub channel_breakdown: Vec<ChannelRevenueDto>,
}

#[async_trait]
pub trait AnalyticsContract: Send + Sync {
    async fn get_sales_analytics(&self) -> Result<SalesAnalyticsDto, ContractError>;
}
