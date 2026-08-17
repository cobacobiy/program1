use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use uuid::Uuid;

use program1_contracts::{
    ContractError, CreateOrderRequest, CreateProductRequest, CreateUserRequest,
    OrderContract, ProductContract, UserContract,
};
use program1_core::init_tracing;
use program1_module_order::OrderModule;
use program1_module_product::ProductModule;
use program1_module_user::UserModule;

#[derive(Clone)]
pub struct AppState {
    pub user_contract: Arc<dyn UserContract>,
    pub product_contract: Arc<dyn ProductContract>,
    pub order_contract: Arc<dyn OrderContract>,
}

#[tokio::main]
async fn main() {
    init_tracing();

    // Instantiate domain modules and wire trait contracts
    let user_module = Arc::new(UserModule::new());
    let product_module = Arc::new(ProductModule::new());
    let order_module = Arc::new(OrderModule::new(
        user_module.clone(),
        product_module.clone(),
    ));

    let state = AppState {
        user_contract: user_module,
        product_contract: product_module,
        order_contract: order_module,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_routes = Router::new()
        // Health
        .route("/health", get(health_check))
        // Users
        .route("/api/v1/users", get(list_users).post(create_user))
        .route("/api/v1/users/:id", get(get_user))
        // Products
        .route("/api/v1/products", get(list_products).post(create_product))
        .route("/api/v1/products/:id", get(get_product))
        // Orders
        .route("/api/v1/orders", get(list_orders).post(create_order))
        .route("/api/v1/orders/:id", get(get_order))
        .with_state(state);

    // Static UI server with fallback to API routes
    let app = Router::new()
        .merge(api_routes)
        .nest_service("/", ServeDir::new("crates/web/static"))
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("Starting Program1 Rust Modular Monolith on http://{}", addr);

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
        ContractError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": msg }))),
    }
}

// User Handlers
async fn list_users(State(state): State<AppState>) -> impl IntoResponse {
    match state.user_contract.list_users().await {
        Ok(users) => (StatusCode::OK, Json(json!(users))),
        Err(e) => map_contract_error(e),
    }
}

async fn get_user(Path(id): Path<Uuid>, State(state): State<AppState>) -> impl IntoResponse {
    match state.user_contract.get_user(id).await {
        Ok(user) => (StatusCode::OK, Json(json!(user))),
        Err(e) => map_contract_error(e),
    }
}

async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> impl IntoResponse {
    match state.user_contract.create_user(payload).await {
        Ok(user) => (StatusCode::CREATED, Json(json!(user))),
        Err(e) => map_contract_error(e),
    }
}

// Product Handlers
async fn list_products(State(state): State<AppState>) -> impl IntoResponse {
    match state.product_contract.list_products().await {
        Ok(products) => (StatusCode::OK, Json(json!(products))),
        Err(e) => map_contract_error(e),
    }
}

async fn get_product(Path(id): Path<Uuid>, State(state): State<AppState>) -> impl IntoResponse {
    match state.product_contract.get_product(id).await {
        Ok(product) => (StatusCode::OK, Json(json!(product))),
        Err(e) => map_contract_error(e),
    }
}

async fn create_product(
    State(state): State<AppState>,
    Json(payload): Json<CreateProductRequest>,
) -> impl IntoResponse {
    match state.product_contract.create_product(payload).await {
        Ok(product) => (StatusCode::CREATED, Json(json!(product))),
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

async fn create_order(
    State(state): State<AppState>,
    Json(payload): Json<CreateOrderRequest>,
) -> impl IntoResponse {
    match state.order_contract.create_order(payload).await {
        Ok(order) => (StatusCode::CREATED, Json(json!(order))),
        Err(e) => map_contract_error(e),
    }
}
