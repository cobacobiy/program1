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
    #[error("Internal module error: {0}")]
    Internal(String),
}

// --- USER CONTRACT DTOs & TRAIT ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDto {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
    pub role: Option<String>,
}

#[async_trait]
pub trait UserContract: Send + Sync {
    async fn get_user(&self, id: Uuid) -> Result<UserDto, ContractError>;
    async fn create_user(&self, req: CreateUserRequest) -> Result<UserDto, ContractError>;
    async fn list_users(&self) -> Result<Vec<UserDto>, ContractError>;
}

// --- PRODUCT CONTRACT DTOs & TRAIT ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductDto {
    pub id: Uuid,
    pub name: String,
    pub sku: String,
    pub price: f64,
    pub stock: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProductRequest {
    pub name: String,
    pub sku: String,
    pub price: f64,
    pub stock: u32,
}

#[async_trait]
pub trait ProductContract: Send + Sync {
    async fn get_product(&self, id: Uuid) -> Result<ProductDto, ContractError>;
    async fn create_product(&self, req: CreateProductRequest) -> Result<ProductDto, ContractError>;
    async fn list_products(&self) -> Result<Vec<ProductDto>, ContractError>;
    async fn reserve_stock(&self, product_id: Uuid, quantity: u32) -> Result<(), ContractError>;
}

// --- ORDER CONTRACT DTOs & TRAIT ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItemDto {
    pub product_id: Uuid,
    pub quantity: u32,
    pub unit_price: f64,
    pub total_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderDto {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    pub items: Vec<OrderItemDto>,
    pub total_amount: f64,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItemRequest {
    pub product_id: Uuid,
    pub quantity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrderRequest {
    pub user_id: Uuid,
    pub items: Vec<OrderItemRequest>,
}

#[async_trait]
pub trait OrderContract: Send + Sync {
    async fn create_order(&self, req: CreateOrderRequest) -> Result<OrderDto, ContractError>;
    async fn get_order(&self, id: Uuid) -> Result<OrderDto, ContractError>;
    async fn list_orders(&self) -> Result<Vec<OrderDto>, ContractError>;
}
