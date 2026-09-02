use std::sync::OnceLock;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

static USERNAME_REGEX: OnceLock<Regex> = OnceLock::new();

pub fn validate_username_regex(username: &str) -> Result<(), validator::ValidationError> {
    let re = USERNAME_REGEX.get_or_init(|| Regex::new(r"^[a-zA-Z0-9_]+$").unwrap());
    if re.is_match(username) {
        Ok(())
    } else {
        let mut err = validator::ValidationError::new("username_format");
        err.message = Some("Username must be alphanumeric or underscore only".into());
        Err(err)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ErrorCode {
    // 400
    ValidationFailed,
    InvalidRequest,

    // 401
    AuthenticationRequired,
    InvalidCredentials,
    TokenExpired,

    // 403
    InsufficientPermissions,

    // 404
    ResourceNotFound,

    // 409
    InsufficientStock,
    DuplicateResource,

    // 429
    RateLimitExceeded,

    // 500
    InternalError,

    // 502
    ChannelSyncFailed,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::ValidationFailed => "VALIDATION_FAILED",
            ErrorCode::InvalidRequest => "INVALID_REQUEST",
            ErrorCode::AuthenticationRequired => "AUTHENTICATION_REQUIRED",
            ErrorCode::InvalidCredentials => "INVALID_CREDENTIALS",
            ErrorCode::TokenExpired => "TOKEN_EXPIRED",
            ErrorCode::InsufficientPermissions => "INSUFFICIENT_PERMISSIONS",
            ErrorCode::ResourceNotFound => "RESOURCE_NOT_FOUND",
            ErrorCode::InsufficientStock => "INSUFFICIENT_STOCK",
            ErrorCode::DuplicateResource => "DUPLICATE_RESOURCE",
            ErrorCode::RateLimitExceeded => "RATE_LIMIT_EXCEEDED",
            ErrorCode::InternalError => "INTERNAL_ERROR",
            ErrorCode::ChannelSyncFailed => "CHANNEL_SYNC_FAILED",
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

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

impl ContractError {
    pub fn code(&self) -> ErrorCode {
        match self {
            ContractError::NotFound(_) => ErrorCode::ResourceNotFound,
            ContractError::ValidationError(_) => ErrorCode::ValidationFailed,
            ContractError::InsufficientStock { .. } => ErrorCode::InsufficientStock,
            ContractError::ChannelSyncError(_) => ErrorCode::ChannelSyncFailed,
            ContractError::Internal(_) => ErrorCode::InternalError,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
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

// --- USER & RBAC PERMISSION CONTRACT ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserAccountDto {
    pub id: Uuid,
    pub username: String,
    pub full_name: String,
    pub role: String,
    pub accessible_menus: Vec<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Request DTO untuk login
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[validate(length(min = 1, max = 50, message = "Username required (1-50 characters)"))]
    pub username: String,
    #[validate(length(min = 1, max = 100, message = "Password required"))]
    pub password: String,
}

/// Response DTO setelah login berhasil
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthTokenResponse {
    pub access_token: String,
    pub token_type: String, // "Bearer"
    pub expires_in: u64,    // seconds
    pub user: UserAccountDto,
}

/// Request DTO untuk register (extend CreateUserAccountRequest)
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct RegisterUserRequest {
    #[validate(
        length(min = 3, max = 50, message = "Username must be 3-50 characters"),
        custom(function = "validate_username_regex")
    )]
    pub username: String,
    #[validate(length(min = 8, max = 100, message = "Password must be at least 8 characters"))]
    pub password: String,
    #[validate(length(min = 1, max = 200, message = "Full name required (max 200 characters)"))]
    pub full_name: String,
    #[validate(length(max = 50))]
    pub role: String,
    pub accessible_menus: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateUserAccountRequest {
    #[validate(
        length(min = 3, max = 50, message = "Username must be 3-50 characters"),
        custom(function = "validate_username_regex")
    )]
    pub username: String,
    #[validate(length(min = 1, max = 200, message = "Full name required (max 200 characters)"))]
    pub full_name: String,
    #[validate(length(min = 1, max = 50, message = "Role required (max 50 characters)"))]
    pub role: String,
    pub accessible_menus: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateUserPermissionsRequest {
    pub accessible_menus: Vec<String>,
}

#[async_trait]
pub trait UserContract: Send + Sync {
    async fn list_accounts(&self) -> Result<Vec<UserAccountDto>, ContractError>;
    async fn get_account(&self, id: Uuid) -> Result<UserAccountDto, ContractError>;
    async fn create_account(&self, req: CreateUserAccountRequest) -> Result<UserAccountDto, ContractError>;
    async fn update_permissions(&self, id: Uuid, accessible_menus: Vec<String>) -> Result<UserAccountDto, ContractError>;

    /// Authenticate user — returns account if credentials valid
    async fn authenticate(&self, username: &str, password: &str) -> Result<UserAccountDto, ContractError>;

    /// Register user baru dengan password
    async fn register(&self, req: RegisterUserRequest) -> Result<UserAccountDto, ContractError>;
}

// --- AUTH & JWT CONTRACT ---

/// JWT Claims structure
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JwtClaims {
    pub sub: Uuid,           // user_id
    pub username: String,
    pub role: String,
    pub accessible_menus: Vec<String>,
    pub exp: i64,            // expiry timestamp
    pub iat: i64,            // issued at
}

#[async_trait]
pub trait AuthContract: Send + Sync {
    /// Generate JWT token dari UserAccountDto
    fn generate_token(&self, user: &UserAccountDto) -> Result<String, ContractError>;

    /// Validate & decode JWT token
    fn validate_token(&self, token: &str) -> Result<JwtClaims, ContractError>;
}

// --- CATALOG CONTRACT ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateCatalogItemRequest {
    #[validate(length(min = 1, max = 200, message = "Name must be 1-200 characters"))]
    pub name: String,
    #[validate(length(min = 1, max = 50, message = "SKU must be 1-50 characters"))]
    pub sku: String,
    #[validate(length(max = 100, message = "Category max 100 characters"))]
    pub category: String,
    #[validate(range(min = 0.0, max = 999999999.0, message = "Price must be between 0 and 999,999,999"))]
    pub price: f64,
    #[validate(range(max = 999999, message = "Stock cannot exceed 999,999"))]
    pub stock: u32,
    #[validate(url(message = "Invalid image URL format"))]
    pub image_url: Option<String>,
    #[validate(length(max = 2000, message = "Description max 2000 characters"))]
    pub description: Option<String>,
}

#[async_trait]
pub trait CatalogContract: Send + Sync {
    async fn list_items(&self) -> Result<Vec<CatalogItemDto>, ContractError>;
    async fn get_item(&self, id: Uuid) -> Result<CatalogItemDto, ContractError>;
    async fn create_item(&self, req: CreateCatalogItemRequest) -> Result<CatalogItemDto, ContractError>;
}

// --- INVENTORY CONTRACT (Ginee OMS Multi-Stock) ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SafetyStockLogDto {
    pub id: Uuid,
    pub product_id: Uuid,
    pub old_safety_stock: u32,
    pub new_safety_stock: u32,
    pub admin_note: String,
    pub updated_by: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateSafetyStockRequest {
    #[validate(range(max = 999999, message = "Safety stock cannot exceed 999,999"))]
    pub new_safety_stock: u32,
    #[validate(length(min = 1, max = 500, message = "Catatan Admin wajib diisi (max 500 karakter)"))]
    pub admin_note: String,
    #[validate(length(max = 100))]
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateWarehouseStockRequest {
    #[validate(range(max = 999999, message = "Stok gudang tidak boleh melebihi 999,999"))]
    pub new_warehouse_stock: u32,
    #[validate(length(min = 1, max = 500, message = "Catatan Admin wajib diisi (max 500 karakter)"))]
    pub admin_note: String,
    #[validate(length(max = 100))]
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateSpareStockRequest {
    #[validate(range(max = 999999, message = "Stok cadangan tidak boleh melebihi 999,999"))]
    pub new_spare_stock: u32,
    #[validate(length(min = 1, max = 500, message = "Catatan Admin wajib diisi (max 500 karakter)"))]
    pub admin_note: String,
    #[validate(length(max = 100))]
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdatePromotionStockRequest {
    #[validate(range(max = 999999, message = "Stok promosi tidak boleh melebihi 999,999"))]
    pub new_promotion_stock: u32,
    #[validate(length(min = 1, max = 500, message = "Catatan Admin wajib diisi (max 500 karakter)"))]
    pub admin_note: String,
    #[validate(length(max = 100))]
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StockAdjustmentLogDto {
    pub id: Uuid,
    pub product_id: Uuid,
    pub adjustment_type: String, // "warehouse", "safety", "spare", "promotion"
    pub old_value: u32,
    pub new_value: u32,
    pub admin_note: String,
    pub updated_by: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LowStockAlertDto {
    pub product_id: Uuid,
    pub product_name: String,
    pub sku: String,
    pub available_stock: u32,
    pub safety_stock: u32,
    pub deficit: u32,
    pub severity: String, // "critical", "warning", "caution"
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct BulkStockAdjustmentItem {
    pub product_id: Uuid,
    #[validate(length(min = 1, max = 20, message = "Tipe stok wajib diisi (warehouse/safety/spare/promotion)"))]
    pub stock_type: String,
    #[validate(range(max = 999999, message = "Nilai stok tidak boleh melebihi 999,999"))]
    pub new_value: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct BulkStockUpdateRequest {
    #[validate(length(min = 1, max = 100, message = "Batch harus berisi 1-100 item"), nested)]
    pub adjustments: Vec<BulkStockAdjustmentItem>,
    #[validate(length(min = 1, max = 500, message = "Catatan Admin wajib diisi (max 500 karakter)"))]
    pub admin_note: String,
    #[validate(length(max = 100))]
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BulkStockUpdateResult {
    pub total_requested: u32,
    pub total_success: u32,
    pub total_failed: u32,
    pub errors: Vec<String>,
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
    async fn update_warehouse_stock(
        &self,
        product_id: Uuid,
        new_warehouse_stock: u32,
        admin_note: String,
        updated_by: String,
    ) -> Result<InventoryStockDto, ContractError>;
    async fn update_spare_stock(
        &self,
        product_id: Uuid,
        new_spare_stock: u32,
        admin_note: String,
        updated_by: String,
    ) -> Result<InventoryStockDto, ContractError>;
    async fn update_promotion_stock(
        &self,
        product_id: Uuid,
        new_promotion_stock: u32,
        admin_note: String,
        updated_by: String,
    ) -> Result<InventoryStockDto, ContractError>;
    async fn get_adjustment_logs(
        &self,
        product_id: Uuid,
        adjustment_type: Option<&str>,
    ) -> Result<Vec<StockAdjustmentLogDto>, ContractError>;
    async fn get_low_stock_alerts(&self) -> Result<Vec<LowStockAlertDto>, ContractError>;
    async fn bulk_update_stock(
        &self,
        request: BulkStockUpdateRequest,
    ) -> Result<BulkStockUpdateResult, ContractError>;
}

// --- CHANNEL SYNC CONTRACT ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrderItemDto {
    pub product_id: Uuid,
    pub product_name: String,
    pub quantity: u32,
    pub unit_price: f64,
    pub total_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct StorefrontOrderItemRequest {
    pub product_id: Uuid,
    #[validate(range(min = 1, max = 9999, message = "Quantity must be 1-9999"))]
    pub quantity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct StorefrontOrderRequest {
    #[validate(length(min = 1, max = 200, message = "Customer name required (max 200 chars)"))]
    pub customer_name: String,
    #[validate(email(message = "Invalid email format"))]
    pub customer_email: String,
    #[validate(length(min = 1, max = 500, message = "Shipping address required (max 500 chars)"))]
    pub shipping_address: String,
    #[validate(length(min = 1, max = 50, message = "Order must have 1-50 items"), nested)]
    pub items: Vec<StorefrontOrderItemRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct MarketplaceOrderReq {
    #[validate(length(min = 1, max = 50))]
    pub channel: String,
    #[validate(length(min = 1, max = 200))]
    pub customer_name: String,
    #[validate(length(min = 1, max = 50), nested)]
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChannelRevenueDto {
    pub channel: ChannelType,
    pub channel_name: String,
    pub total_orders: u32,
    pub total_revenue: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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

// --- AUDIT LOGGING CONTRACT ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub actor_id: Option<Uuid>,
    pub actor_username: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    pub details: String,
    pub ip_address: Option<String>,
}

#[async_trait]
pub trait AuditContract: Send + Sync {
    async fn log_action(&self, entry: AuditLogEntry) -> Result<(), ContractError>;
    async fn get_logs(
        &self,
        resource_type: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AuditLogEntry>, ContractError>;
    async fn get_logs_by_actor(&self, actor_id: Uuid) -> Result<Vec<AuditLogEntry>, ContractError>;
}
