use async_trait::async_trait;
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
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

    // 413
    PayloadTooLarge,

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
            ErrorCode::PayloadTooLarge => "PAYLOAD_TOO_LARGE",
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
    #[error(
        "Insufficient stock for product {product_id}: requested {requested}, available {available}"
    )]
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
    #[validate(length(
        min = 1,
        max = 200,
        message = "Full name required (max 200 characters)"
    ))]
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
    #[validate(length(
        min = 1,
        max = 200,
        message = "Full name required (max 200 characters)"
    ))]
    pub full_name: String,
    #[validate(length(min = 1, max = 50, message = "Role required (max 50 characters)"))]
    pub role: String,
    pub accessible_menus: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateUserPermissionsRequest {
    pub accessible_menus: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BreakGlassStatusDto {
    pub is_active: bool,
    pub active_until: Option<DateTime<Utc>>,
    pub activated_by: Option<String>,
    pub reason: Option<String>,
    pub target_account: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct ActivateBreakGlassRequest {
    #[validate(length(
        min = 5,
        max = 500,
        message = "Reason for break-glass emergency access required (5-500 chars)"
    ))]
    pub reason: String,
    #[validate(range(
        min = 5,
        max = 240,
        message = "Duration must be between 5 and 240 minutes"
    ))]
    pub duration_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct DeactivateBreakGlassRequest {
    pub reason: Option<String>,
}

#[async_trait]
pub trait UserContract: Send + Sync {
    async fn list_accounts(&self) -> Result<Vec<UserAccountDto>, ContractError>;
    async fn get_account(&self, id: Uuid) -> Result<UserAccountDto, ContractError>;
    async fn create_account(
        &self,
        req: CreateUserAccountRequest,
    ) -> Result<UserAccountDto, ContractError>;
    async fn update_permissions(
        &self,
        id: Uuid,
        accessible_menus: Vec<String>,
    ) -> Result<UserAccountDto, ContractError>;

    /// Authenticate user — returns account if credentials valid
    async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<UserAccountDto, ContractError>;

    /// Register user baru dengan password
    async fn register(&self, req: RegisterUserRequest) -> Result<UserAccountDto, ContractError>;

    /// Break-glass emergency developer access activation
    async fn activate_break_glass(
        &self,
        req: ActivateBreakGlassRequest,
        activated_by: &str,
    ) -> Result<BreakGlassStatusDto, ContractError>;
    async fn deactivate_break_glass(
        &self,
        deactivated_by: &str,
        reason: Option<String>,
    ) -> Result<BreakGlassStatusDto, ContractError>;
    async fn get_break_glass_status(&self) -> Result<BreakGlassStatusDto, ContractError>;
}

// --- AUTH & JWT CONTRACT ---

fn default_seller_user_type() -> String {
    "seller_staff".to_string()
}

/// JWT Claims structure
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JwtClaims {
    pub sub: Uuid, // user_id or buyer_id
    pub username: String,
    pub role: String,
    pub accessible_menus: Vec<String>,
    pub exp: i64, // expiry timestamp
    pub iat: i64, // issued at
    #[serde(default = "default_seller_user_type")]
    pub user_type: String, // "buyer" | "seller_staff"
}

impl JwtClaims {
    pub fn is_buyer(&self) -> bool {
        self.user_type == "buyer"
    }

    pub fn is_seller_staff(&self) -> bool {
        self.user_type == "seller_staff"
    }
}

#[async_trait]
pub trait AuthContract: Send + Sync {
    /// Generate JWT token dari UserAccountDto (Seller/Staff)
    fn generate_token(&self, user: &UserAccountDto) -> Result<String, ContractError>;

    /// Generate JWT token dari BuyerAccountDto (Buyer Domain)
    fn generate_buyer_token(&self, buyer: &BuyerAccountDto) -> Result<String, ContractError>;

    /// Validate & decode JWT token
    fn validate_token(&self, token: &str) -> Result<JwtClaims, ContractError>;
}

// --- PAGINATION COMMON MODELS ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct PaginationParams {
    #[validate(range(min = 1, max = 1000, message = "Page must be between 1 and 1000"))]
    pub page: Option<i64>,
    #[validate(range(min = 1, max = 100, message = "Page size must be between 1 and 100"))]
    pub page_size: Option<i64>,
    #[validate(length(max = 200, message = "Search query max 200 characters"))]
    pub search: Option<String>,
    #[validate(length(max = 100, message = "Category filter max 100 characters"))]
    pub category: Option<String>,
    #[validate(length(max = 50, message = "Status filter max 50 characters"))]
    pub status: Option<String>,
    #[validate(length(max = 50, message = "Sort by max 50 characters"))]
    pub sort_by: Option<String>,
    #[validate(length(max = 10, message = "Sort order max 10 characters"))]
    pub sort_order: Option<String>,
}

impl PaginationParams {
    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn page_size(&self) -> i64 {
        self.page_size.unwrap_or(20).clamp(1, 100)
    }

    pub fn offset(&self) -> i64 {
        (self.page() - 1) * self.page_size()
    }
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
    #[validate(range(
        min = 0.0,
        max = 999999999.0,
        message = "Price must be between 0 and 999,999,999"
    ))]
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

    /// Paginated listing with search, category filtering, and sorting
    async fn list_items_paginated(
        &self,
        page: i64,
        page_size: i64,
        search: Option<&str>,
        category: Option<&str>,
        sort_by: Option<&str>,
        sort_order: Option<&str>,
    ) -> Result<PaginatedResponse<CatalogItemDto>, ContractError>;

    async fn get_item(&self, id: Uuid) -> Result<CatalogItemDto, ContractError>;
    async fn create_item(
        &self,
        req: CreateCatalogItemRequest,
    ) -> Result<CatalogItemDto, ContractError>;
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
    #[validate(length(max = 500, message = "Catatan Admin max 500 karakter"))]
    pub admin_note: String,
    #[validate(length(max = 100))]
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateWarehouseStockRequest {
    #[validate(range(max = 999999, message = "Stok gudang tidak boleh melebihi 999,999"))]
    pub new_warehouse_stock: u32,
    #[validate(length(max = 500, message = "Catatan Admin max 500 karakter"))]
    pub admin_note: String,
    #[validate(length(max = 100))]
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateSpareStockRequest {
    #[validate(range(max = 999999, message = "Stok cadangan tidak boleh melebihi 999,999"))]
    pub new_spare_stock: u32,
    #[validate(length(max = 500, message = "Catatan Admin max 500 karakter"))]
    pub admin_note: String,
    #[validate(length(max = 100))]
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdatePromotionStockRequest {
    #[validate(range(max = 999999, message = "Stok promosi tidak boleh melebihi 999,999"))]
    pub new_promotion_stock: u32,
    #[validate(length(max = 500, message = "Catatan Admin max 500 karakter"))]
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
    #[validate(length(
        min = 1,
        max = 20,
        message = "Tipe stok wajib diisi (warehouse/safety/spare/promotion)"
    ))]
    pub stock_type: String,
    #[validate(range(max = 999999, message = "Nilai stok tidak boleh melebihi 999,999"))]
    pub new_value: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct BulkStockUpdateRequest {
    #[validate(
        length(min = 1, max = 100, message = "Batch harus berisi 1-100 item"),
        nested
    )]
    pub adjustments: Vec<BulkStockAdjustmentItem>,
    #[validate(length(max = 500, message = "Catatan Admin max 500 karakter"))]
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

    /// Paginated stock listing with search and sorting
    async fn list_all_paginated(
        &self,
        page: i64,
        page_size: i64,
        search: Option<&str>,
        sort_by: Option<&str>,
        sort_order: Option<&str>,
    ) -> Result<PaginatedResponse<InventoryStockDto>, ContractError>;

    async fn get_stock(&self, product_id: Uuid) -> Result<InventoryStockDto, ContractError>;
    async fn reserve_stock(&self, product_id: Uuid, quantity: u32) -> Result<(), ContractError>;
    async fn update_safety_stock(
        &self,
        product_id: Uuid,
        new_safety_stock: u32,
        admin_note: String,
        updated_by: String,
    ) -> Result<InventoryStockDto, ContractError>;
    async fn get_safety_stock_logs(
        &self,
        product_id: Uuid,
    ) -> Result<Vec<SafetyStockLogDto>, ContractError>;
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
pub struct ShippingAddressSnapshot {
    pub recipient_name: String,
    pub phone_number: String,
    pub street_address: String,
    pub subdistrict: String,
    pub city: String,
    pub province: String,
    pub postal_code: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    Pending,
    Paid,
    Processing,
    Shipped,
    Delivered,
    Completed,
    Cancelled,
    ReturnRequested,
    Returned,
}

impl OrderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Paid => "paid",
            Self::Processing => "processing",
            Self::Shipped => "shipped",
            Self::Delivered => "delivered",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::ReturnRequested => "return_requested",
            Self::Returned => "returned",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "pending" => Some(Self::Pending),
            "paid" => Some(Self::Paid),
            "processing" => Some(Self::Processing),
            "shipped" => Some(Self::Shipped),
            "delivered" => Some(Self::Delivered),
            "completed" => Some(Self::Completed),
            "cancelled" => Some(Self::Cancelled),
            "return_requested" => Some(Self::ReturnRequested),
            "returned" => Some(Self::Returned),
            _ => None,
        }
    }

    /// Allowed transitions from the current order status
    pub fn allowed_transitions(&self) -> Vec<OrderStatus> {
        match self {
            Self::Pending => vec![Self::Paid, Self::Cancelled],
            Self::Paid => vec![Self::Processing, Self::Cancelled],
            Self::Processing => vec![Self::Shipped],
            Self::Shipped => vec![Self::Delivered],
            Self::Delivered => vec![Self::Completed, Self::ReturnRequested],
            Self::ReturnRequested => vec![Self::Returned, Self::Completed],
            _ => vec![],
        }
    }

    pub fn can_transition_to(&self, target: &OrderStatus) -> bool {
        self.allowed_transitions().contains(target)
    }
}

impl std::fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
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
    #[serde(default)]
    pub buyer_id: Option<Uuid>,
    #[serde(default)]
    pub shipping_snapshot: Option<ShippingAddressSnapshot>,
    #[serde(default)]
    pub tracking_number: Option<String>,
    #[serde(default)]
    pub shipped_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub delivered_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub cancelled_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub cancelled_by: Option<String>,
    #[serde(default)]
    pub cancel_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateOrderStatusRequest {
    #[serde(alias = "status")]
    pub new_status: OrderStatus,
    #[validate(length(max = 100, message = "Nomor resi max 100 karakter"))]
    pub tracking_number: Option<String>,
    #[serde(alias = "cancel_reason")]
    #[validate(length(max = 500, message = "Alasan max 500 karakter"))]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Validate, ToSchema)]
pub struct CancelOrderRequest {
    #[validate(length(max = 500, message = "Alasan pembatalan max 500 karakter"))]
    pub reason: Option<String>,
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
    #[validate(length(
        min = 1,
        max = 500,
        message = "Shipping address required (max 500 chars)"
    ))]
    pub shipping_address: String,
    #[validate(
        length(min = 1, max = 50, message = "Order must have 1-50 items"),
        nested
    )]
    pub items: Vec<StorefrontOrderItemRequest>,
    pub buyer_id: Option<Uuid>,
    pub shipping_snapshot: Option<ShippingAddressSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct BuyerCheckoutRequest {
    pub address_id: Uuid,
    #[validate(
        length(min = 1, max = 50, message = "Order must have 1-50 items"),
        nested
    )]
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
    async fn create_storefront_order(
        &self,
        req: StorefrontOrderRequest,
    ) -> Result<OmniOrderDto, ContractError>;
    async fn create_marketplace_order(
        &self,
        channel: ChannelType,
        customer_name: String,
        items: Vec<StorefrontOrderItemRequest>,
    ) -> Result<OmniOrderDto, ContractError>;
    async fn list_orders(&self) -> Result<Vec<OmniOrderDto>, ContractError>;

    /// Paginated orders listing with status filtering and sorting
    async fn list_orders_paginated(
        &self,
        page: i64,
        page_size: i64,
        status: Option<&str>,
        sort_by: Option<&str>,
        sort_order: Option<&str>,
    ) -> Result<PaginatedResponse<OmniOrderDto>, ContractError>;

    async fn get_order(&self, id: Uuid) -> Result<OmniOrderDto, ContractError>;

    /// Update order status with transition validation
    async fn update_order_status(
        &self,
        order_id: Uuid,
        new_status: OrderStatus,
        updated_by: &str,
    ) -> Result<OmniOrderDto, ContractError>;

    /// Update order status with additional metadata (tracking number, timestamps, reason)
    async fn update_order_status_with_metadata(
        &self,
        order_id: Uuid,
        new_status: OrderStatus,
        updated_by: &str,
        tracking_number: Option<String>,
        reason: Option<String>,
    ) -> Result<OmniOrderDto, ContractError>;

    /// List orders placed by a specific buyer
    async fn list_buyer_orders(&self, buyer_id: Uuid) -> Result<Vec<OmniOrderDto>, ContractError>;
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

// --- BUYER & ADDRESS CONTRACT ---

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BuyerAccountDto {
    pub id: Uuid,
    #[serde(default)]
    pub google_sub: Option<String>,
    pub email: String,
    pub full_name: String,
    pub avatar_url: Option<String>,
    pub phone_number: Option<String>,
    pub phone_verified: bool,
    #[serde(default = "default_true")]
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct RegisterBuyerRequest {
    #[validate(length(
        min = 2,
        max = 100,
        message = "Nama lengkap minimal 2 karakter (maks 100)"
    ))]
    pub full_name: String,
    #[validate(email(message = "Format email tidak valid"))]
    pub email: String,
    #[validate(length(min = 8, max = 100, message = "Kata sandi minimal 8 karakter"))]
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct BuyerLoginRequest {
    #[validate(email(message = "Format email tidak valid"))]
    pub email: String,
    #[validate(length(min = 1, message = "Kata sandi tidak boleh kosong"))]
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateBuyerStatusRequest {
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateBuyerProfileRequest {
    #[validate(length(
        min = 2,
        max = 100,
        message = "Nama lengkap minimal 2 karakter (maks 100)"
    ))]
    pub full_name: Option<String>,
    #[validate(url(message = "Format URL avatar tidak valid"))]
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct GoogleAuthRequest {
    #[validate(length(min = 1, message = "Google ID token is required"))]
    pub id_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BuyerAuthResponse {
    #[serde(alias = "token")]
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub buyer: BuyerAccountDto,
    pub requires_phone_verification: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct OtpRequest {
    #[validate(length(min = 8, max = 20, message = "Valid phone number required"))]
    pub phone_number: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct OtpVerifyRequest {
    #[validate(length(min = 8, max = 20, message = "Phone number is required"))]
    pub phone_number: String,
    #[serde(alias = "otp_code")]
    #[validate(length(min = 4, max = 8, message = "OTP code must be 4-8 digits"))]
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BuyerAddressDto {
    pub id: Uuid,
    pub buyer_id: Uuid,
    pub recipient_name: String,
    pub phone_number: String,
    pub street_address: String,
    pub subdistrict: String,
    pub city: String,
    pub province: String,
    pub postal_code: String,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateBuyerAddressRequest {
    #[validate(length(
        min = 1,
        max = 200,
        message = "Recipient name required (max 200 chars)"
    ))]
    pub recipient_name: String,
    #[validate(length(min = 8, max = 25, message = "Valid recipient phone number required"))]
    pub phone_number: String,
    #[validate(length(min = 3, max = 500, message = "Street address required"))]
    pub street_address: String,
    #[validate(length(min = 1, max = 100, message = "Subdistrict / Kecamatan required"))]
    pub subdistrict: String,
    #[validate(length(min = 1, max = 100, message = "City / Kota required"))]
    pub city: String,
    #[validate(length(min = 1, max = 100, message = "Province / Provinsi required"))]
    pub province: String,
    #[validate(length(min = 3, max = 10, message = "Valid postal code required"))]
    pub postal_code: String,
    #[serde(default, alias = "is_default")]
    pub set_as_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateBuyerAddressRequest {
    #[validate(length(
        min = 1,
        max = 200,
        message = "Recipient name required (max 200 chars)"
    ))]
    pub recipient_name: String,
    #[validate(length(min = 8, max = 25, message = "Valid recipient phone number required"))]
    pub phone_number: String,
    #[validate(length(min = 3, max = 500, message = "Street address required"))]
    pub street_address: String,
    #[validate(length(min = 1, max = 100, message = "Subdistrict / Kecamatan required"))]
    pub subdistrict: String,
    #[validate(length(min = 1, max = 100, message = "City / Kota required"))]
    pub city: String,
    #[validate(length(min = 1, max = 100, message = "Province / Provinsi required"))]
    pub province: String,
    #[validate(length(min = 3, max = 10, message = "Valid postal code required"))]
    pub postal_code: String,
    #[serde(default, alias = "is_default")]
    pub set_as_default: bool,
}

#[async_trait]
pub trait BuyerContract: Send + Sync {
    async fn register(&self, req: RegisterBuyerRequest)
        -> Result<BuyerAuthResponse, ContractError>;
    async fn login(&self, req: BuyerLoginRequest) -> Result<BuyerAuthResponse, ContractError>;
    async fn authenticate_google(&self, id_token: &str)
        -> Result<BuyerAuthResponse, ContractError>;
    async fn request_phone_otp(
        &self,
        buyer_id: Uuid,
        phone_number: &str,
    ) -> Result<Option<String>, ContractError>;
    async fn verify_phone_otp(
        &self,
        buyer_id: Uuid,
        phone_number: &str,
        code: &str,
    ) -> Result<BuyerAccountDto, ContractError>;
    async fn get_buyer_profile(&self, buyer_id: Uuid) -> Result<BuyerAccountDto, ContractError>;
    async fn list_addresses(&self, buyer_id: Uuid) -> Result<Vec<BuyerAddressDto>, ContractError>;
    async fn get_address(
        &self,
        buyer_id: Uuid,
        address_id: Uuid,
    ) -> Result<BuyerAddressDto, ContractError>;
    async fn create_address(
        &self,
        buyer_id: Uuid,
        req: CreateBuyerAddressRequest,
    ) -> Result<BuyerAddressDto, ContractError>;
    async fn update_address(
        &self,
        buyer_id: Uuid,
        address_id: Uuid,
        req: UpdateBuyerAddressRequest,
    ) -> Result<BuyerAddressDto, ContractError>;
    async fn delete_address(&self, buyer_id: Uuid, address_id: Uuid) -> Result<(), ContractError>;
    async fn set_default_address(
        &self,
        buyer_id: Uuid,
        address_id: Uuid,
    ) -> Result<BuyerAddressDto, ContractError>;
    async fn list_all_buyers(&self) -> Result<Vec<BuyerAccountDto>, ContractError>;

    /// Paginated buyer accounts listing with search query
    async fn list_buyers_paginated(
        &self,
        page: i64,
        page_size: i64,
        search: Option<&str>,
    ) -> Result<PaginatedResponse<BuyerAccountDto>, ContractError>;

    async fn set_buyer_active_status(
        &self,
        buyer_id: Uuid,
        is_active: bool,
    ) -> Result<BuyerAccountDto, ContractError>;

    async fn update_buyer_profile(
        &self,
        buyer_id: Uuid,
        full_name: Option<String>,
        avatar_url: Option<String>,
    ) -> Result<BuyerAccountDto, ContractError>;
}

// --- LIVE CHAT & MESSAGING CONTRACT (FUTURE-PROOF ARCHITECTURE) ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ChatSenderType {
    Buyer,
    Seller,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChatMessageDto {
    pub id: Uuid,
    pub room_id: Uuid,
    pub sender_type: ChatSenderType,
    pub sender_id: Uuid,
    pub sender_name: String,
    pub content: String,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChatRoomDto {
    pub id: Uuid,
    pub buyer_id: Uuid,
    pub buyer_name: String,
    pub last_message: Option<String>,
    pub unread_count: i64,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct SendMessageRequest {
    #[validate(length(
        min = 1,
        max = 2000,
        message = "Pesan obrolan harus antara 1 sampai 2000 karakter"
    ))]
    pub content: String,
}

#[async_trait]
pub trait ChatContract: Send + Sync {
    async fn send_message(
        &self,
        room_id: Uuid,
        sender_type: ChatSenderType,
        sender_id: Uuid,
        sender_name: &str,
        content: &str,
    ) -> Result<ChatMessageDto, ContractError>;

    async fn get_messages(
        &self,
        room_id: Uuid,
        limit: i64,
    ) -> Result<Vec<ChatMessageDto>, ContractError>;

    async fn get_or_create_buyer_room(
        &self,
        buyer_id: Uuid,
        buyer_name: &str,
    ) -> Result<ChatRoomDto, ContractError>;

    async fn list_active_rooms(&self) -> Result<Vec<ChatRoomDto>, ContractError>;
}

// --- PAYMENT GATEWAY CONTRACT (MIDTRANS) ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaymentTransactionDto {
    pub id: Uuid,
    pub order_id: Uuid,
    pub payment_method: String,
    pub amount: f64,
    pub currency: String,
    pub status: String,
    pub provider_ref: Option<String>,
    pub snap_token: Option<String>,
    pub snap_redirect_url: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreatePaymentRequest {
    pub order_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaymentConfigDto {
    pub client_key: String,
    pub is_production: bool,
    pub snap_url: String,
}

#[async_trait]
pub trait PaymentContract: Send + Sync {
    /// Create payment intent — calls Midtrans Snap API, returns snap_token
    async fn create_payment(
        &self,
        order_id: Uuid,
        amount: f64,
        customer_name: &str,
        customer_email: &str,
    ) -> Result<PaymentTransactionDto, ContractError>;

    /// Handle webhook notification from Midtrans
    async fn handle_notification(
        &self,
        payload: serde_json::Value,
    ) -> Result<PaymentTransactionDto, ContractError>;

    /// Get payment status by order_id
    async fn get_payment_by_order(
        &self,
        order_id: Uuid,
    ) -> Result<Option<PaymentTransactionDto>, ContractError>;

    /// Get client-facing Midtrans configuration
    fn get_config(&self) -> PaymentConfigDto;
}
