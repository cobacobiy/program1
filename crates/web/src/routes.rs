use std::time::Duration;

use axum::{
    extract::DefaultBodyLimit,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, patch, post, put},
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
            rate_limit::rate_limit_layer(lim, "auth_login", 5, Duration::from_secs(60), req, next)
                .await
        }
    });

    let buyer_auth_limiter = limiter.clone();
    let buyer_auth_limit_layer = axum::middleware::from_fn(move |req, next| {
        let lim = buyer_auth_limiter.clone();
        async move {
            rate_limit::rate_limit_layer(
                lim,
                "buyer_auth_google",
                10,
                Duration::from_secs(60),
                req,
                next,
            )
            .await
        }
    });

    let buyer_login_limiter = limiter.clone();
    let buyer_login_limit_layer = axum::middleware::from_fn(move |req, next| {
        let lim = buyer_login_limiter.clone();
        async move {
            rate_limit::rate_limit_layer(
                lim,
                "buyer_auth_login",
                5,
                Duration::from_secs(60),
                req,
                next,
            )
            .await
        }
    });

    let otp_limiter = limiter.clone();
    let otp_limit_layer = axum::middleware::from_fn(move |req, next| {
        let lim = otp_limiter.clone();
        async move {
            rate_limit::rate_limit_layer(lim, "buyer_otp", 5, Duration::from_secs(60), req, next)
                .await
        }
    });

    let register_limiter = limiter.clone();
    let register_limit_layer = axum::middleware::from_fn(move |req, next| {
        let lim = register_limiter.clone();
        async move {
            rate_limit::rate_limit_layer(
                lim,
                "auth_register",
                3,
                Duration::from_secs(60),
                req,
                next,
            )
            .await
        }
    });

    let order_limiter = limiter.clone();
    let order_limit_layer = axum::middleware::from_fn(move |req, next| {
        let lim = order_limiter.clone();
        async move {
            rate_limit::rate_limit_layer(lim, "orders", 10, Duration::from_secs(60), req, next)
                .await
        }
    });

    let catalog_limiter = limiter.clone();
    let catalog_limit_layer = axum::middleware::from_fn(move |req, next| {
        let lim = catalog_limiter.clone();
        async move {
            rate_limit::rate_limit_layer(lim, "catalog", 20, Duration::from_secs(60), req, next)
                .await
        }
    });

    let inventory_limiter = limiter.clone();
    let inventory_limit_layer = axum::middleware::from_fn(move |req, next| {
        let lim = inventory_limiter.clone();
        async move {
            rate_limit::rate_limit_layer(
                lim,
                "inventory_mutation",
                10,
                Duration::from_secs(60),
                req,
                next,
            )
            .await
        }
    });

    let chat_limiter = limiter.clone();
    let chat_limit_layer = axum::middleware::from_fn(move |req, next| {
        let lim = chat_limiter.clone();
        async move {
            rate_limit::rate_limit_layer(lim, "chat_messages", 30, Duration::from_secs(60), req, next)
                .await
        }
    });

    let payment_limiter = limiter.clone();
    let payment_limit_layer = axum::middleware::from_fn(move |req, next| {
        let lim = payment_limiter.clone();
        async move {
            rate_limit::rate_limit_layer(lim, "payments", 30, Duration::from_secs(60), req, next)
                .await
        }
    });

    let upload_limiter = limiter.clone();
    let upload_limit_layer = axum::middleware::from_fn(move |req, next| {
        let lim = upload_limiter.clone();
        async move {
            rate_limit::rate_limit_layer(lim, "uploads", 10, Duration::from_secs(60), req, next)
                .await
        }
    });

    // 1. Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/health/ready", get(readiness_check))
        .route("/api/v1/store/info", get(get_store_info))
        .route(
            "/api/v1/payments/config",
            get(get_payment_config_handler),
        )
        .route(
            "/api/v1/payments/notification",
            post(payment_notification_handler).route_layer(payment_limit_layer.clone()),
        )
        .route(
            "/api/v1/buyer/auth/config",
            get(get_buyer_auth_config_handler),
        )
        .route(
            "/api/v1/auth/login",
            post(login_handler).route_layer(login_limit_layer),
        )
        .route(
            "/api/v1/buyer/auth/google",
            post(google_auth_handler).route_layer(buyer_auth_limit_layer),
        )
        .route(
            "/api/v1/buyer/auth/register",
            post(buyer_register_handler).route_layer(register_limit_layer.clone()),
        )
        .route(
            "/api/v1/buyer/auth/login",
            post(buyer_login_handler).route_layer(buyer_login_limit_layer),
        )
        .route("/api/v1/catalog", get(list_catalog))
        .route("/api/v1/catalog/:id", get(get_catalog_item))
        .route("/api/v1/catalog/:id/variants", get(list_variants_handler))
        .route("/api/v1/catalog/:id/variants/:vid", get(get_variant_handler));

    // 2. Buyer protected routes (valid Buyer JWT required)
    let buyer_routes = Router::new()
        .route(
            "/api/v1/buyer/otp/request",
            post(request_otp_handler).route_layer(otp_limit_layer.clone()),
        )
        .route(
            "/api/v1/buyer/otp/verify",
            post(verify_otp_handler).route_layer(otp_limit_layer),
        )
        .route(
            "/api/v1/buyer/profile",
            get(get_buyer_profile_handler).put(update_buyer_profile_handler),
        )
        .route(
            "/api/v1/buyer/addresses",
            get(list_addresses_handler).post(create_address_handler),
        )
        .route(
            "/api/v1/buyer/addresses/:id",
            put(update_address_handler).delete(delete_address_handler),
        )
        .route(
            "/api/v1/buyer/addresses/:id/default",
            post(set_default_address_handler),
        )
        .route(
            "/api/v1/orders",
            post(create_storefront_order).route_layer(order_limit_layer.clone()),
        )
        .route(
            "/api/v1/buyer/orders",
            get(list_buyer_orders_handler),
        )
        .route(
            "/api/v1/buyer/orders/:id",
            get(get_buyer_order_handler),
        )
        .route(
            "/api/v1/orders/:id/cancel",
            post(buyer_cancel_order_handler).route_layer(order_limit_layer.clone()),
        )
        .route(
            "/api/v1/orders/:id/confirm-delivery",
            post(buyer_confirm_delivery_handler).route_layer(order_limit_layer.clone()),
        )
        .route("/api/v1/chat/rooms", post(buyer_get_or_create_room_handler))
        .route(
            "/api/v1/chat/rooms/:room_id/messages",
            get(buyer_get_messages_handler)
                .post(buyer_send_message_handler)
                .route_layer(chat_limit_layer.clone()),
        )
        .route(
            "/api/v1/payments",
            post(buyer_create_payment_handler).route_layer(payment_limit_layer.clone()),
        )
        .route(
            "/api/v1/payments/order/:order_id",
            get(get_payment_by_order_handler),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::require_buyer_auth,
        ));

    // 3. Seller Protected routes (valid Seller/Staff JWT authentication required)
    let protected_routes = Router::new()
        .route(
            "/api/v1/uploads/images",
            post(upload_image_handler)
                .route_layer(upload_limit_layer)
                .route_layer(DefaultBodyLimit::max(5 * 1024 * 1024)),
        )
        .route(
            "/api/v1/catalog",
            post(create_catalog_item).route_layer(catalog_limit_layer.clone()),
        )
        .route(
            "/api/v1/catalog/:id/variants",
            post(create_variant_handler).route_layer(catalog_limit_layer.clone()),
        )
        .route(
            "/api/v1/catalog/:id/variants/:vid",
            put(update_variant_handler)
                .delete(delete_variant_handler)
                .route_layer(catalog_limit_layer),
        )
        .route("/api/v1/inventory", get(list_all_inventory))
        .route(
            "/api/v1/inventory/alerts/low-stock",
            get(get_low_stock_alerts),
        )
        .route("/api/v1/inventory/:id", get(get_inventory_stock))
        .route(
            "/api/v1/inventory/:id/safety-stock-logs",
            get(get_safety_stock_logs),
        )
        .route(
            "/api/v1/inventory/:id/adjustment-logs",
            get(get_adjustment_logs),
        )
        .route("/api/v1/channels", get(list_channels))
        .route("/api/v1/channels/sync/:channel", post(sync_channel))
        .route("/api/v1/orders", get(list_orders))
        .route("/api/v1/orders/:id", get(get_order))
        .route(
            "/api/v1/orders/:id/status",
            patch(update_order_status_handler).route_layer(order_limit_layer.clone()),
        )
        .route(
            "/api/v1/orders/marketplace",
            post(create_marketplace_order).route_layer(order_limit_layer),
        )
        .route("/api/v1/users/accounts", get(list_user_accounts))
        .route("/api/v1/admin/buyers", get(admin_list_buyers_handler))
        .route(
            "/api/v1/admin/buyers/:id/status",
            patch(admin_set_buyer_status_handler),
        )
        .route(
            "/api/v1/admin/buyers/activity",
            get(admin_list_buyer_activity_handler),
        )
        .route("/api/v1/admin/chat/rooms", get(admin_list_chat_rooms_handler))
        .route(
            "/api/v1/admin/chat/rooms/:room_id/messages",
            get(admin_get_messages_handler)
                .post(admin_send_message_handler)
                .route_layer(chat_limit_layer),
        )
        .route(
            "/api/v1/admin/payments/order/:order_id",
            get(get_payment_by_order_handler),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::require_auth,
        ));

    // 4. Admin-only routes (valid Seller JWT with admin role required)
    let admin_routes = Router::new()
        .route(
            "/api/v1/auth/register",
            post(register_handler).route_layer(register_limit_layer),
        )
        .route("/api/v1/users/accounts", post(create_user_account))
        .route(
            "/api/v1/users/accounts/:id/permissions",
            post(update_user_permissions),
        )
        .route(
            "/api/v1/inventory/bulk-update",
            post(bulk_update_stock).route_layer(inventory_limit_layer.clone()),
        )
        .route(
            "/api/v1/inventory/:id/safety-stock",
            post(update_safety_stock).route_layer(inventory_limit_layer.clone()),
        )
        .route(
            "/api/v1/inventory/:id/warehouse-stock",
            post(update_warehouse_stock).route_layer(inventory_limit_layer.clone()),
        )
        .route(
            "/api/v1/inventory/:id/spare-stock",
            post(update_spare_stock).route_layer(inventory_limit_layer.clone()),
        )
        .route(
            "/api/v1/inventory/:id/promotion-stock",
            post(update_promotion_stock).route_layer(inventory_limit_layer),
        )
        .route("/api/v1/analytics", get(get_analytics))
        .route("/api/v1/audit/logs", get(list_audit_logs))
        .route("/api/v1/audit/logs/user/:id", get(get_user_audit_logs))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::require_admin,
        ));

    // 5. Merchant Owner / Super Admin only routes (Break-Glass emergency access)
    let break_glass_routes = Router::new()
        .route(
            "/api/v1/admin/break-glass/activate",
            post(activate_break_glass),
        )
        .route(
            "/api/v1/admin/break-glass/deactivate",
            post(deactivate_break_glass),
        )
        .route(
            "/api/v1/admin/break-glass/status",
            get(get_break_glass_status),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::require_seller_owner,
        ));

    // Static pages
    let admin_page = std::fs::read_to_string("crates/web/static/index.html")
        .unwrap_or_else(|_| "<h1>Admin Hub</h1>".to_string());
    let store_page = std::fs::read_to_string("crates/web/static/store.html")
        .unwrap_or_else(|_| "<h1>Storefront</h1>".to_string());
    let store_page_alt = store_page.clone();

    // Svelte SPA page support (if built)
    let svelte_page = std::fs::read_to_string("crates/web/static/dist/index.html").ok();
    let root_page = svelte_page.clone().unwrap_or(store_page);

    let assets_service = ServeDir::new("crates/web/static/dist/assets")
        .fallback(ServeDir::new("crates/web/static"));

    let mut static_routes = Router::new()
        .route("/admin", get(move || async move { Html(admin_page) }))
        .route("/store", get(move || async move { Html(store_page_alt) }))
        .route("/", get(move || async move { Html(root_page) }))
        .nest_service("/assets", assets_service)
        .nest_service("/uploads", ServeDir::new("data/uploads"));

    if let Some(sp) = svelte_page {
        static_routes = static_routes.route("/svelte", get(move || async move { Html(sp) }));
    }

    // Swagger UI & OpenAPI Specification routes
    let doc_routes = SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi());

    Router::new()
        .merge(public_routes)
        .merge(buyer_routes)
        .merge(protected_routes)
        .merge(admin_routes)
        .merge(break_glass_routes)
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
