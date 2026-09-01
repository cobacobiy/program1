use std::time::Duration;

use axum::{
    extract::DefaultBodyLimit,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::services::ServeDir;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::docs::ApiDoc;
use crate::error::ApiError;
use crate::handlers::*;
use crate::middleware;
use crate::rate_limit;
use crate::state::AppState;
use program1_contracts::ErrorCode;

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
        .route("/health/ready", get(readiness_check))
        .route("/api/v1/store/info", get(get_store_info))
        .route("/api/v1/auth/login", post(login_handler).route_layer(login_limit_layer))
        .route("/api/v1/catalog", get(list_catalog))
        .route("/api/v1/catalog/:id", get(get_catalog_item))
        .route("/api/v1/orders", post(create_storefront_order).route_layer(order_limit_layer.clone()));

    // 2. Protected routes (valid JWT authentication required)
    let protected_routes = Router::new()
        .route("/api/v1/catalog", post(create_catalog_item).route_layer(catalog_limit_layer))
        .route("/api/v1/inventory", get(list_all_inventory))
        .route("/api/v1/inventory/bulk-update", post(bulk_update_stock))
        .route("/api/v1/inventory/alerts/low-stock", get(get_low_stock_alerts))
        .route("/api/v1/inventory/:id", get(get_inventory_stock))
        .route("/api/v1/inventory/:id/safety-stock", post(update_safety_stock))
        .route("/api/v1/inventory/:id/safety-stock-logs", get(get_safety_stock_logs))
        .route("/api/v1/inventory/:id/warehouse-stock", post(update_warehouse_stock))
        .route("/api/v1/inventory/:id/spare-stock", post(update_spare_stock))
        .route("/api/v1/inventory/:id/promotion-stock", post(update_promotion_stock))
        .route("/api/v1/inventory/:id/adjustment-logs", get(get_adjustment_logs))
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

    // Swagger UI & OpenAPI Specification routes
    let doc_routes = SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi());

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(admin_routes)
        .merge(static_routes)
        .merge(doc_routes)
        .layer(CatchPanicLayer::custom(|panic_info| {
            tracing::error!("Handler panicked! Error: {:?}", panic_info);
            let err = ApiError::new(
                ErrorCode::InternalError,
                "An unexpected internal error occurred",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
            err.into_response()
        }))
        .layer(axum::middleware::from_fn(middleware::security_headers))
        .layer(axum::middleware::from_fn(middleware::request_id_middleware))
        .layer(DefaultBodyLimit::max(1024 * 1024)) // 1MB max body limit
        .layer(cors)
        .with_state(state)
}
