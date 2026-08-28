pub mod error;
pub mod middleware;
pub mod rate_limit;

pub use error::ApiError;

use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{DefaultBodyLimit, FromRequest, Path, Query, Request, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::json;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::services::ServeDir;
use uuid::Uuid;
use validator::Validate;

use program1_contracts::{
    AnalyticsContract, AuditContract, AuditLogEntry, AuthContract, AuthTokenResponse,
    CatalogContract, CatalogItemDto, ChannelRevenueDto, ChannelStatusDto, ChannelSyncContract,
    ChannelType, CreateCatalogItemRequest, CreateUserAccountRequest, ErrorCode,
    InventoryContract, InventoryStockDto, LoginRequest, MarketplaceOrderReq, OmniOrderDto,
    OrderContract, RegisterUserRequest, SafetyStockLogDto, SalesAnalyticsDto, StorefrontOrderRequest,
    UpdateSafetyStockRequest, UpdateUserPermissionsRequest, UserAccountDto, UserContract,
};

#[derive(Clone)]
pub struct AppState {
    pub store_name: String,
    pub store_currency: String,
    pub user_contract: Arc<dyn UserContract>,
    pub auth_contract: Arc<dyn AuthContract>,
    pub catalog_contract: Arc<dyn CatalogContract>,
    pub inventory_contract: Arc<dyn InventoryContract>,
    pub channel_contract: Arc<dyn ChannelSyncContract>,
    pub order_contract: Arc<dyn OrderContract>,
    pub analytics_contract: Arc<dyn AnalyticsContract>,
    pub audit_contract: Arc<dyn AuditContract>,
    pub rate_limiter: Arc<rate_limit::IpRateLimiter>,
}

/// Custom Axum extractor that parses JSON and automatically runs validation
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedJson<T>(pub T);

#[axum::async_trait]
impl<T> FromRequest<AppState> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &AppState) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| ApiError::new(ErrorCode::InvalidRequest, e.to_string(), StatusCode::BAD_REQUEST))?;

        value.validate().map_err(|e| {
            let mut details = Vec::new();
            for (field, err_kind) in e.field_errors() {
                for err in err_kind {
                    let msg = err
                        .message
                        .as_ref()
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| format!("Invalid value for {}", field));
                    details.push(format!("{}: {}", field, msg));
                }
            }

            ApiError::with_details(
                ErrorCode::ValidationFailed,
                "Payload validation failed",
                StatusCode::UNPROCESSABLE_ENTITY,
                details,
            )
        })?;

        Ok(ValidatedJson(value))
    }
}

pub fn create_app(state: AppState) -> Router {
    let cors = middleware::build_cors_layer();
    let limiter = state.rate_limiter.clone();

    // Specific rate limit layers
    let login_limiter = limiter.clone();
    let login_limit_layer = axum::middleware::from_fn(move |req, next| {
        let lim = login_limiter.clone();
        async move {
            rate_limit::rate_limit_layer(lim, "auth_login", 5, Duration::from_secs(60), req, next).await
        }
    });

    let register_limiter = limiter.clone();
    let register_limit_layer = axum::middleware::from_fn(move |req, next| {
        let lim = register_limiter.clone();
        async move {
            rate_limit::rate_limit_layer(lim, "auth_register", 3, Duration::from_secs(60), req, next).await
        }
    });

    let order_limiter = limiter.clone();
    let order_limit_layer = axum::middleware::from_fn(move |req, next| {
        let lim = order_limiter.clone();
        async move {
            rate_limit::rate_limit_layer(lim, "orders", 10, Duration::from_secs(60), req, next).await
        }
    });

    let catalog_limiter = limiter.clone();
    let catalog_limit_layer = axum::middleware::from_fn(move |req, next| {
        let lim = catalog_limiter.clone();
        async move {
            rate_limit::rate_limit_layer(lim, "catalog", 20, Duration::from_secs(60), req, next).await
        }
    });

    // 1. Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/store/info", get(get_store_info))
        .route("/api/v1/auth/login", post(login_handler).route_layer(login_limit_layer))
        .route("/api/v1/catalog", get(list_catalog))
        .route("/api/v1/catalog/:id", get(get_catalog_item))
        .route("/api/v1/orders", post(create_storefront_order).route_layer(order_limit_layer.clone()));

    // 2. Protected routes (valid JWT authentication required)
    let protected_routes = Router::new()
        .route("/api/v1/catalog", post(create_catalog_item).route_layer(catalog_limit_layer))
        .route("/api/v1/inventory", get(list_all_inventory))
        .route("/api/v1/inventory/:id", get(get_inventory_stock))
        .route("/api/v1/inventory/:id/safety-stock", post(update_safety_stock))
        .route("/api/v1/inventory/:id/safety-stock-logs", get(get_safety_stock_logs))
        .route("/api/v1/channels", get(list_channels))
        .route("/api/v1/channels/sync/:channel", post(sync_channel))
        .route("/api/v1/orders", get(list_orders))
        .route("/api/v1/orders/:id", get(get_order))
        .route("/api/v1/orders/marketplace", post(create_marketplace_order).route_layer(order_limit_layer))
        .route_layer(axum::middleware::from_fn_with_state(state.clone(), middleware::require_auth));

    // 3. Admin-only routes (valid JWT with admin role required)
    let admin_routes = Router::new()
        .route("/api/v1/auth/register", post(register_handler).route_layer(register_limit_layer))
        .route("/api/v1/users/accounts", get(list_user_accounts).post(create_user_account))
        .route("/api/v1/users/accounts/:id/permissions", post(update_user_permissions))
        .route("/api/v1/analytics", get(get_analytics))
        .route("/api/v1/audit/logs", get(list_audit_logs))
        .route("/api/v1/audit/logs/user/:id", get(get_user_audit_logs))
        .route_layer(axum::middleware::from_fn_with_state(state.clone(), middleware::require_admin));

    // Static pages
    let admin_page = std::fs::read_to_string("crates/web/static/index.html")
        .unwrap_or_else(|_| "<h1>Admin Hub</h1>".to_string());
    let store_page = std::fs::read_to_string("crates/web/static/store.html")
        .unwrap_or_else(|_| "<h1>Storefront</h1>".to_string());
    let store_page_alt = store_page.clone();

    let static_routes = Router::new()
        .route("/admin", get(move || async move { Html(admin_page) }))
        .route("/store", get(move || async move { Html(store_page_alt) }))
        .route("/", get(move || async move { Html(store_page) }))
        .nest_service("/assets", ServeDir::new("crates/web/static"));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(admin_routes)
        .merge(static_routes)
        .layer(CatchPanicLayer::custom(|_| {
            let err = ApiError::new(
                ErrorCode::InternalError,
                "An unexpected internal error occurred",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
            err.into_response()
        }))
        .layer(axum::middleware::from_fn(middleware::security_headers))
        .layer(DefaultBodyLimit::max(1024 * 1024)) // 1MB max body limit
        .layer(cors)
        .with_state(state)
}

// --- HANDLERS ---

pub async fn health_check() -> impl IntoResponse {
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

pub async fn get_store_info(State(state): State<AppState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "store_name": state.store_name,
            "currency": state.store_currency,
        })),
    )
}

// Auth Handlers
pub async fn login_handler(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<LoginRequest>,
) -> Result<Json<AuthTokenResponse>, ApiError> {
    match state.user_contract.authenticate(&payload.username, &payload.password).await {
        Ok(user) => {
            let token = state.auth_contract.generate_token(&user)?;

            let _ = state.audit_contract.log_action(AuditLogEntry {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                actor_id: Some(user.id),
                actor_username: user.username.clone(),
                action: "LOGIN_SUCCESS".to_string(),
                resource_type: "user".to_string(),
                resource_id: Some(user.id),
                details: json!({ "role": user.role }).to_string(),
                ip_address: None,
            }).await;

            let token_resp = AuthTokenResponse {
                access_token: token,
                token_type: "Bearer".to_string(),
                expires_in: 86400,
                user,
            };
            Ok(Json(token_resp))
        }
        Err(e) => {
            let _ = state.audit_contract.log_action(AuditLogEntry {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                actor_id: None,
                actor_username: payload.username.clone(),
                action: "LOGIN_FAILED".to_string(),
                resource_type: "user".to_string(),
                resource_id: None,
                details: json!({ "attempted_username": payload.username }).to_string(),
                ip_address: None,
            }).await;

            Err(ApiError::from(e))
        }
    }
}

pub async fn register_handler(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<RegisterUserRequest>,
) -> Result<(StatusCode, Json<UserAccountDto>), ApiError> {
    let user = state.user_contract.register(payload).await?;

    let _ = state.audit_contract.log_action(AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        actor_id: Some(user.id),
        actor_username: user.username.clone(),
        action: "USER_REGISTERED".to_string(),
        resource_type: "user".to_string(),
        resource_id: Some(user.id),
        details: json!({ "role": user.role }).to_string(),
        ip_address: None,
    }).await;

    Ok((StatusCode::CREATED, Json(user)))
}

// User Accounts & RBAC Handlers
pub async fn list_user_accounts(State(state): State<AppState>) -> Result<Json<Vec<UserAccountDto>>, ApiError> {
    let accounts = state.user_contract.list_accounts().await?;
    Ok(Json(accounts))
}

pub async fn create_user_account(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateUserAccountRequest>,
) -> Result<(StatusCode, Json<UserAccountDto>), ApiError> {
    let acc = state.user_contract.create_account(payload).await?;

    let _ = state.audit_contract.log_action(AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        actor_id: Some(acc.id),
        actor_username: acc.username.clone(),
        action: "USER_CREATED".to_string(),
        resource_type: "user".to_string(),
        resource_id: Some(acc.id),
        details: json!({ "role": acc.role }).to_string(),
        ip_address: None,
    }).await;

    Ok((StatusCode::CREATED, Json(acc)))
}

pub async fn update_user_permissions(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<UpdateUserPermissionsRequest>,
) -> Result<Json<UserAccountDto>, ApiError> {
    let acc = state.user_contract.update_permissions(id, payload.accessible_menus.clone()).await?;

    let _ = state.audit_contract.log_action(AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        actor_id: Some(id),
        actor_username: acc.username.clone(),
        action: "PERMISSIONS_UPDATED".to_string(),
        resource_type: "user".to_string(),
        resource_id: Some(id),
        details: json!({ "accessible_menus": payload.accessible_menus }).to_string(),
        ip_address: None,
    }).await;

    Ok(Json(acc))
}

// Catalog Handlers
pub async fn list_catalog(State(state): State<AppState>) -> Result<Json<Vec<CatalogItemDto>>, ApiError> {
    let items = state.catalog_contract.list_items().await?;
    Ok(Json(items))
}

pub async fn get_catalog_item(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<CatalogItemDto>, ApiError> {
    let item = state.catalog_contract.get_item(id).await?;
    Ok(Json(item))
}

pub async fn create_catalog_item(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateCatalogItemRequest>,
) -> Result<(StatusCode, Json<CatalogItemDto>), ApiError> {
    let sku = payload.sku.clone();
    let item = state.catalog_contract.create_item(payload).await?;

    let _ = state.audit_contract.log_action(AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        actor_id: None,
        actor_username: "admin".to_string(),
        action: "CATALOG_ITEM_CREATED".to_string(),
        resource_type: "catalog".to_string(),
        resource_id: Some(item.id),
        details: json!({ "sku": sku, "name": item.name }).to_string(),
        ip_address: None,
    }).await;

    Ok((StatusCode::CREATED, Json(item)))
}

// Inventory Ginee OMS Handlers
pub async fn list_all_inventory(State(state): State<AppState>) -> Result<Json<Vec<InventoryStockDto>>, ApiError> {
    let stocks = state.inventory_contract.get_all_stocks().await?;
    Ok(Json(stocks))
}

pub async fn get_inventory_stock(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<InventoryStockDto>, ApiError> {
    let stock = state.inventory_contract.get_stock(id).await?;
    Ok(Json(stock))
}

pub async fn update_safety_stock(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<UpdateSafetyStockRequest>,
) -> Result<Json<InventoryStockDto>, ApiError> {
    let operator = payload.updated_by.unwrap_or_else(|| "Admin Ginee".to_string());
    let updated = state
        .inventory_contract
        .update_safety_stock(id, payload.new_safety_stock, payload.admin_note.clone(), operator.clone())
        .await?;

    let _ = state.audit_contract.log_action(AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        actor_id: None,
        actor_username: operator,
        action: "SAFETY_STOCK_UPDATED".to_string(),
        resource_type: "inventory".to_string(),
        resource_id: Some(id),
        details: json!({ "new_safety_stock": payload.new_safety_stock, "note": payload.admin_note }).to_string(),
        ip_address: None,
    }).await;

    Ok(Json(updated))
}

pub async fn get_safety_stock_logs(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<Vec<SafetyStockLogDto>>, ApiError> {
    let logs = state.inventory_contract.get_safety_stock_logs(id).await?;
    Ok(Json(logs))
}

// Channel Sync Handlers
pub async fn list_channels(State(state): State<AppState>) -> Result<Json<Vec<ChannelStatusDto>>, ApiError> {
    let channels = state.channel_contract.get_channel_statuses().await?;
    Ok(Json(channels))
}

pub async fn sync_channel(
    Path(channel_name): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let channel = match channel_name.to_lowercase().as_str() {
        "tiktok" | "tiktokshop" => ChannelType::TikTokShop,
        "shopee" => ChannelType::Shopee,
        "tokopedia" => ChannelType::Tokopedia,
        _ => ChannelType::NativeWeb,
    };

    let count = state.channel_contract.sync_channel_stock(channel.clone()).await?;

    let _ = state.audit_contract.log_action(AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        actor_id: None,
        actor_username: "system".to_string(),
        action: "CHANNEL_SYNCED".to_string(),
        resource_type: "channel".to_string(),
        resource_id: None,
        details: json!({ "channel": channel.to_string(), "synced_products": count }).to_string(),
        ip_address: None,
    }).await;

    Ok(Json(json!({ "synced_products": count })))
}

// Order Handlers
pub async fn list_orders(State(state): State<AppState>) -> Result<Json<Vec<OmniOrderDto>>, ApiError> {
    let orders = state.order_contract.list_orders().await?;
    Ok(Json(orders))
}

pub async fn get_order(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<OmniOrderDto>, ApiError> {
    let order = state.order_contract.get_order(id).await?;
    Ok(Json(order))
}

pub async fn create_storefront_order(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<StorefrontOrderRequest>,
) -> Result<(StatusCode, Json<OmniOrderDto>), ApiError> {
    let order = state.order_contract.create_storefront_order(payload).await?;

    let _ = state.audit_contract.log_action(AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        actor_id: None,
        actor_username: order.customer_name.clone(),
        action: "ORDER_CREATED".to_string(),
        resource_type: "order".to_string(),
        resource_id: Some(order.id),
        details: json!({ "total_amount": order.total_amount, "channel": "NativeWeb" }).to_string(),
        ip_address: None,
    }).await;

    Ok((StatusCode::CREATED, Json(order)))
}

pub async fn create_marketplace_order(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<MarketplaceOrderReq>,
) -> Result<(StatusCode, Json<OmniOrderDto>), ApiError> {
    let channel = match payload.channel.to_lowercase().as_str() {
        "tiktok" | "tiktokshop" => ChannelType::TikTokShop,
        "shopee" => ChannelType::Shopee,
        "tokopedia" => ChannelType::Tokopedia,
        _ => ChannelType::NativeWeb,
    };

    let order = state
        .order_contract
        .create_marketplace_order(channel.clone(), payload.customer_name.clone(), payload.items)
        .await?;

    let _ = state.audit_contract.log_action(AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        actor_id: None,
        actor_username: order.customer_name.clone(),
        action: "ORDER_CREATED".to_string(),
        resource_type: "order".to_string(),
        resource_id: Some(order.id),
        details: json!({ "total_amount": order.total_amount, "channel": channel.to_string() }).to_string(),
        ip_address: None,
    }).await;

    Ok((StatusCode::CREATED, Json(order)))
}

// Analytics Handlers
pub async fn get_analytics(State(state): State<AppState>) -> Result<Json<SalesAnalyticsDto>, ApiError> {
    let analytics = state.analytics_contract.get_sales_analytics().await?;
    Ok(Json(analytics))
}

// Audit Logs Query Handlers
#[derive(Debug, Deserialize)]
pub struct AuditQueryParam {
    pub resource_type: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

pub async fn list_audit_logs(
    State(state): State<AppState>,
    Query(params): Query<AuditQueryParam>,
) -> Result<Json<Vec<AuditLogEntry>>, ApiError> {
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);
    let res_type = params.resource_type.as_deref();

    let logs = state.audit_contract.get_logs(res_type, limit, offset).await?;
    Ok(Json(logs))
}

pub async fn get_user_audit_logs(
    Path(user_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<Vec<AuditLogEntry>>, ApiError> {
    let logs = state.audit_contract.get_logs_by_actor(user_id).await?;
    Ok(Json(logs))
}
