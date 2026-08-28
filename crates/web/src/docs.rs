use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};

use crate::error::ApiError;
use crate::handlers;
use program1_contracts::{
    AuditLogEntry, AuthTokenResponse, CatalogItemDto, ChannelRevenueDto, ChannelStatusDto,
    ChannelType, CreateCatalogItemRequest, CreateUserAccountRequest, ErrorCode,
    InventoryStockDto, JwtClaims, LoginRequest, MarketplaceOrderReq, OmniOrderDto, OrderItemDto,
    RegisterUserRequest, SafetyStockLogDto, SalesAnalyticsDto, StorefrontOrderItemRequest,
    StorefrontOrderRequest, UpdateSafetyStockRequest, UpdateUserPermissionsRequest, UserAccountDto,
};

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some("Enter your Bearer JWT token"))
                        .build(),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::health_check,
        handlers::get_store_info,
        handlers::login_handler,
        handlers::register_handler,
        handlers::list_user_accounts,
        handlers::create_user_account,
        handlers::update_user_permissions,
        handlers::list_catalog,
        handlers::get_catalog_item,
        handlers::create_catalog_item,
        handlers::list_all_inventory,
        handlers::get_inventory_stock,
        handlers::update_safety_stock,
        handlers::get_safety_stock_logs,
        handlers::list_channels,
        handlers::sync_channel,
        handlers::list_orders,
        handlers::get_order,
        handlers::create_storefront_order,
        handlers::create_marketplace_order,
        handlers::get_analytics,
        handlers::list_audit_logs,
        handlers::get_user_audit_logs,
    ),
    components(
        schemas(
            ApiError,
            ErrorCode,
            UserAccountDto,
            LoginRequest,
            AuthTokenResponse,
            RegisterUserRequest,
            CreateUserAccountRequest,
            UpdateUserPermissionsRequest,
            JwtClaims,
            CatalogItemDto,
            CreateCatalogItemRequest,
            InventoryStockDto,
            SafetyStockLogDto,
            UpdateSafetyStockRequest,
            ChannelType,
            ChannelStatusDto,
            OrderItemDto,
            OmniOrderDto,
            StorefrontOrderItemRequest,
            StorefrontOrderRequest,
            MarketplaceOrderReq,
            ChannelRevenueDto,
            SalesAnalyticsDto,
            AuditLogEntry,
            handlers::AuditQueryParam,
        )
    ),
    tags(
        (name = "Health", description = "System health check and metadata"),
        (name = "Auth", description = "Authentication & registration endpoints"),
        (name = "Catalog", description = "Product catalog management"),
        (name = "Inventory", description = "Ginee OMS multi-warehouse & safety stock"),
        (name = "Channels", description = "Omnichannel marketplace sync"),
        (name = "Orders", description = "Storefront checkout & order management"),
        (name = "Analytics", description = "Sales analytics and gross revenue metrics"),
        (name = "Users", description = "User accounts and RBAC permissions"),
        (name = "Audit", description = "System audit logs & compliance activity trail"),
    ),
    modifiers(&SecurityAddon),
    info(
        title = "Program1 — Omnichannel Commerce API",
        version = "1.0.0",
        description = "High-performance modular monolith REST API engine for AURA Storefront."
    )
)]
pub struct ApiDoc;
