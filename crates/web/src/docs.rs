use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};

use crate::error::ApiError;
use crate::handlers;
use program1_contracts::{
    AuditLogEntry, AuthTokenResponse, BulkStockAdjustmentItem, BulkStockUpdateRequest,
    BulkStockUpdateResult, BuyerAccountDto, BuyerCheckoutRequest, BuyerLoginRequest,
    CancelOrderRequest, CatalogItemDto, ChannelRevenueDto, ChannelStatusDto, ChannelType,
    ChatMessageDto, ChatRoomDto, ChatSenderType, CreateCatalogItemRequest, CreatePaymentRequest,
    CreateUserAccountRequest, ErrorCode, InventoryStockDto, JwtClaims, LoginRequest,
    LowStockAlertDto, MarketplaceOrderReq, OmniOrderDto, OrderItemDto, OrderStatus,
    PaymentConfigDto, PaymentTransactionDto, RegisterBuyerRequest, RegisterUserRequest,
    SafetyStockLogDto, SalesAnalyticsDto, SendMessageRequest, StockAdjustmentLogDto,
    StorefrontOrderItemRequest, StorefrontOrderRequest, UpdateBuyerStatusRequest,
    UpdateOrderStatusRequest, UpdatePromotionStockRequest, UpdateSafetyStockRequest,
    UpdateSpareStockRequest, UpdateUserPermissionsRequest, UpdateWarehouseStockRequest,
    UserAccountDto, PaginationParams,
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
        handlers::readiness_check,
        handlers::get_store_info,
        handlers::get_buyer_auth_config_handler,

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
        handlers::update_warehouse_stock,
        handlers::update_spare_stock,
        handlers::update_promotion_stock,
        handlers::get_adjustment_logs,
        handlers::get_low_stock_alerts,
        handlers::bulk_update_stock,
        handlers::list_channels,
        handlers::sync_channel,
        handlers::list_orders,
        handlers::get_order,
        handlers::create_storefront_order,
        handlers::create_marketplace_order,
        handlers::update_order_status_handler,
        handlers::buyer_cancel_order_handler,
        handlers::buyer_confirm_delivery_handler,
        handlers::list_buyer_orders_handler,
        handlers::get_analytics,
        handlers::list_audit_logs,
        handlers::get_user_audit_logs,
        handlers::buyer_register_handler,
        handlers::buyer_login_handler,
        handlers::admin_list_buyers_handler,
        handlers::admin_set_buyer_status_handler,
        handlers::admin_list_buyer_activity_handler,
        handlers::buyer_get_or_create_room_handler,
        handlers::buyer_get_messages_handler,
        handlers::buyer_send_message_handler,
        handlers::admin_list_chat_rooms_handler,
        handlers::admin_get_messages_handler,
        handlers::admin_send_message_handler,
        handlers::buyer_create_payment_handler,
        handlers::payment_notification_handler,
        handlers::get_payment_by_order_handler,
        handlers::get_payment_config_handler,
        handlers::upload_image_handler,
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
            UpdateWarehouseStockRequest,
            UpdateSpareStockRequest,
            UpdatePromotionStockRequest,
            StockAdjustmentLogDto,
            LowStockAlertDto,
            BulkStockAdjustmentItem,
            BulkStockUpdateRequest,
            BulkStockUpdateResult,
            ChannelType,
            ChannelStatusDto,
            OrderItemDto,
            OmniOrderDto,
            OrderStatus,
            UpdateOrderStatusRequest,
            CancelOrderRequest,
            BuyerCheckoutRequest,
            StorefrontOrderItemRequest,
            StorefrontOrderRequest,
            MarketplaceOrderReq,
            ChannelRevenueDto,
            SalesAnalyticsDto,
            AuditLogEntry,
            handlers::AuditQueryParam,
            BuyerAccountDto,
            RegisterBuyerRequest,
            BuyerLoginRequest,
            UpdateBuyerStatusRequest,
            ChatMessageDto,
            ChatRoomDto,
            ChatSenderType,
            SendMessageRequest,
            CreatePaymentRequest,
            PaymentTransactionDto,
            PaymentConfigDto,
            handlers::UploadResponse,
            PaginationParams,
        )
    ),
    tags(
        (name = "Health", description = "System health check and metadata"),
        (name = "Auth", description = "Authentication & registration endpoints"),
        (name = "Buyer Auth", description = "Buyer storefront authentication & profile"),
        (name = "Admin Buyers", description = "Admin customer directory & CRM"),
        (name = "Catalog", description = "Product catalog management"),
        (name = "Uploads", description = "Product image upload & file storage"),
        (name = "Inventory", description = "Ginee OMS multi-warehouse & safety stock"),
        (name = "Channels", description = "Omnichannel marketplace sync"),
        (name = "Orders", description = "Storefront checkout & order management"),
        (name = "Payments", description = "Midtrans payment processing & Snap checkout"),
        (name = "Analytics", description = "Sales analytics and gross revenue metrics"),
        (name = "Users", description = "User accounts and RBAC permissions"),
        (name = "Audit", description = "System audit logs & compliance activity trail"),
        (name = "Buyer Live Chat", description = "Buyer in-app chat messaging"),
        (name = "Admin Live Chat", description = "Merchant live chat support inbox"),
    ),
    modifiers(&SecurityAddon),
    info(
        title = "Program1 — Omnichannel Commerce API",
        version = "1.0.0",
        description = "High-performance modular monolith REST API engine for AURA Storefront."
    )
)]
pub struct ApiDoc;
