pub mod middleware;

use std::sync::Arc;

use axum::{
    extract::{DefaultBodyLimit, FromRequest, Path, Request, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use serde::de::DeserializeOwned;
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use uuid::Uuid;
use validator::Validate;

use program1_contracts::{
    AnalyticsContract, AuthContract, AuthTokenResponse, CatalogContract, ChannelSyncContract,
    ChannelType, ContractError, CreateCatalogItemRequest, CreateUserAccountRequest,
    InventoryContract, LoginRequest, MarketplaceOrderReq, OrderContract, RegisterUserRequest,
    StorefrontOrderRequest, UpdateSafetyStockRequest, UpdateUserPermissionsRequest, UserContract,
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
}

/// Custom Axum extractor that parses JSON and automatically runs validation
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedJson<T>(pub T);

#[axum::async_trait]
impl<T> FromRequest<AppState> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
{
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request(req: Request, state: &AppState) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))))?;

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

            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({
                    "error": "validation_failed",
                    "details": details
                })),
            )
        })?;

        Ok(ValidatedJson(value))
    }
}

pub fn create_app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 1. Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/store/info", get(get_store_info))
        .route("/api/v1/auth/login", post(login_handler))
        .route("/api/v1/catalog", get(list_catalog))
        .route("/api/v1/catalog/:id", get(get_catalog_item))
        .route("/api/v1/orders", post(create_storefront_order));

    // 2. Protected routes (valid JWT authentication required)
    let protected_routes = Router::new()
        .route("/api/v1/catalog", post(create_catalog_item))
        .route("/api/v1/inventory", get(list_all_inventory))
        .route("/api/v1/inventory/:id", get(get_inventory_stock))
        .route("/api/v1/inventory/:id/safety-stock", post(update_safety_stock))
        .route("/api/v1/inventory/:id/safety-stock-logs", get(get_safety_stock_logs))
        .route("/api/v1/channels", get(list_channels))
        .route("/api/v1/channels/sync/:channel", post(sync_channel))
        .route("/api/v1/orders", get(list_orders))
        .route("/api/v1/orders/:id", get(get_order))
        .route("/api/v1/orders/marketplace", post(create_marketplace_order))
        .route_layer(axum::middleware::from_fn_with_state(state.clone(), middleware::require_auth));

    // 3. Admin-only routes (valid JWT with admin role required)
    let admin_routes = Router::new()
        .route("/api/v1/auth/register", post(register_handler))
        .route("/api/v1/users/accounts", get(list_user_accounts).post(create_user_account))
        .route("/api/v1/users/accounts/:id/permissions", post(update_user_permissions))
        .route("/api/v1/analytics", get(get_analytics))
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

pub fn map_contract_error(err: ContractError) -> (StatusCode, Json<serde_json::Value>) {
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

// Auth Handlers
pub async fn login_handler(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<LoginRequest>,
) -> impl IntoResponse {
    match state.user_contract.authenticate(&payload.username, &payload.password).await {
        Ok(user) => {
            match state.auth_contract.generate_token(&user) {
                Ok(token) => {
                    let token_resp = AuthTokenResponse {
                        access_token: token,
                        token_type: "Bearer".to_string(),
                        expires_in: 86400,
                        user,
                    };
                    (StatusCode::OK, Json(json!(token_resp)))
                }
                Err(e) => map_contract_error(e),
            }
        }
        Err(e) => map_contract_error(e),
    }
}

pub async fn register_handler(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<RegisterUserRequest>,
) -> impl IntoResponse {
    match state.user_contract.register(payload).await {
        Ok(user) => (StatusCode::CREATED, Json(json!(user))),
        Err(e) => map_contract_error(e),
    }
}

// User Accounts & RBAC Handlers
pub async fn list_user_accounts(State(state): State<AppState>) -> impl IntoResponse {
    match state.user_contract.list_accounts().await {
        Ok(accounts) => (StatusCode::OK, Json(json!(accounts))),
        Err(e) => map_contract_error(e),
    }
}

pub async fn create_user_account(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateUserAccountRequest>,
) -> impl IntoResponse {
    match state.user_contract.create_account(payload).await {
        Ok(acc) => (StatusCode::CREATED, Json(json!(acc))),
        Err(e) => map_contract_error(e),
    }
}

pub async fn update_user_permissions(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<UpdateUserPermissionsRequest>,
) -> impl IntoResponse {
    match state.user_contract.update_permissions(id, payload.accessible_menus).await {
        Ok(acc) => (StatusCode::OK, Json(json!(acc))),
        Err(e) => map_contract_error(e),
    }
}

// Catalog Handlers
pub async fn list_catalog(State(state): State<AppState>) -> impl IntoResponse {
    match state.catalog_contract.list_items().await {
        Ok(items) => (StatusCode::OK, Json(json!(items))),
        Err(e) => map_contract_error(e),
    }
}

pub async fn get_catalog_item(Path(id): Path<Uuid>, State(state): State<AppState>) -> impl IntoResponse {
    match state.catalog_contract.get_item(id).await {
        Ok(item) => (StatusCode::OK, Json(json!(item))),
        Err(e) => map_contract_error(e),
    }
}

pub async fn create_catalog_item(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateCatalogItemRequest>,
) -> impl IntoResponse {
    match state.catalog_contract.create_item(payload).await {
        Ok(item) => (StatusCode::CREATED, Json(json!(item))),
        Err(e) => map_contract_error(e),
    }
}

// Inventory Ginee OMS Handlers
pub async fn list_all_inventory(State(state): State<AppState>) -> impl IntoResponse {
    match state.inventory_contract.get_all_stocks().await {
        Ok(stocks) => (StatusCode::OK, Json(json!(stocks))),
        Err(e) => map_contract_error(e),
    }
}

pub async fn get_inventory_stock(Path(id): Path<Uuid>, State(state): State<AppState>) -> impl IntoResponse {
    match state.inventory_contract.get_stock(id).await {
        Ok(stock) => (StatusCode::OK, Json(json!(stock))),
        Err(e) => map_contract_error(e),
    }
}

pub async fn update_safety_stock(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<UpdateSafetyStockRequest>,
) -> impl IntoResponse {
    let operator = payload.updated_by.unwrap_or_else(|| "Admin Ginee".to_string());
    match state
        .inventory_contract
        .update_safety_stock(id, payload.new_safety_stock, payload.admin_note, operator)
        .await
    {
        Ok(updated) => (StatusCode::OK, Json(json!(updated))),
        Err(e) => map_contract_error(e),
    }
}

pub async fn get_safety_stock_logs(Path(id): Path<Uuid>, State(state): State<AppState>) -> impl IntoResponse {
    match state.inventory_contract.get_safety_stock_logs(id).await {
        Ok(logs) => (StatusCode::OK, Json(json!(logs))),
        Err(e) => map_contract_error(e),
    }
}

// Channel Sync Handlers
pub async fn list_channels(State(state): State<AppState>) -> impl IntoResponse {
    match state.channel_contract.get_channel_statuses().await {
        Ok(channels) => (StatusCode::OK, Json(json!(channels))),
        Err(e) => map_contract_error(e),
    }
}

pub async fn sync_channel(Path(channel_name): Path<String>, State(state): State<AppState>) -> impl IntoResponse {
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
pub async fn list_orders(State(state): State<AppState>) -> impl IntoResponse {
    match state.order_contract.list_orders().await {
        Ok(orders) => (StatusCode::OK, Json(json!(orders))),
        Err(e) => map_contract_error(e),
    }
}

pub async fn get_order(Path(id): Path<Uuid>, State(state): State<AppState>) -> impl IntoResponse {
    match state.order_contract.get_order(id).await {
        Ok(order) => (StatusCode::OK, Json(json!(order))),
        Err(e) => map_contract_error(e),
    }
}

pub async fn create_storefront_order(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<StorefrontOrderRequest>,
) -> impl IntoResponse {
    match state.order_contract.create_storefront_order(payload).await {
        Ok(order) => (StatusCode::CREATED, Json(json!(order))),
        Err(e) => map_contract_error(e),
    }
}

pub async fn create_marketplace_order(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<MarketplaceOrderReq>,
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
pub async fn get_analytics(State(state): State<AppState>) -> impl IntoResponse {
    match state.analytics_contract.get_sales_analytics().await {
        Ok(analytics) => (StatusCode::OK, Json(json!(analytics))),
        Err(e) => map_contract_error(e),
    }
}
