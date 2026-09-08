use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use chrono::Utc;
use program1_contracts::{AuthContract, ChatMessageDto, ChatRoomDto, ChatSenderType, UserContract};
use program1_core::init_database;
use program1_module_analytics::AnalyticsModule;
use program1_module_audit::AuditModule;
use program1_module_auth::AuthModule;
use program1_module_catalog::CatalogModule;
use program1_module_channel::ChannelSyncModule;
use program1_module_chat::ChatModule;
use program1_module_inventory::InventoryModule;
use program1_module_order::OrderModule;
use program1_module_user::UserModule;
use program1_web::{create_app, rate_limit::IpRateLimiter, AppState};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

async fn setup_test_app() -> (axum::Router, String, String, Uuid) {
    let secret = "test-jwt-secret-key-minimum-32-characters-length!".to_string();
    let pool = init_database("sqlite::memory:")
        .await
        .expect("Test DB init failed");

    let user_module = Arc::new(UserModule::new(pool.clone()));
    let auth_module = Arc::new(AuthModule::new(secret, 24));
    let catalog_module = Arc::new(CatalogModule::new(pool.clone()));
    let inventory_module = Arc::new(InventoryModule::new(pool.clone(), catalog_module.clone()));
    let channel_module = Arc::new(ChannelSyncModule::new(pool.clone()));
    let order_module = Arc::new(OrderModule::new(
        pool.clone(),
        catalog_module.clone(),
        inventory_module.clone(),
    ));
    let analytics_module = Arc::new(AnalyticsModule::new(
        catalog_module.clone(),
        order_module.clone(),
    ));
    let audit_module = Arc::new(AuditModule::new(pool.clone()));
    let rate_limiter = Arc::new(IpRateLimiter::new());

    let google_verifier = Arc::new(program1_module_buyer::ProductionGoogleVerifier {
        client_id: "test".to_string(),
    });
    let sms_sender = Arc::new(program1_module_buyer::ConsoleOrProviderSmsSender::default());
    let buyer_module = Arc::new(program1_module_buyer::BuyerModule::new(
        pool.clone(),
        auth_module.clone(),
        google_verifier,
        sms_sender,
        audit_module.clone(),
    ));
    let chat_module = Arc::new(ChatModule::new(pool.clone()));
    let payment_module = Arc::new(program1_module_payment::PaymentModule::new(
        pool.clone(),
        "test-server-key".to_string(),
        "test-client-key".to_string(),
        false,
    ));
    let coupon_module = Arc::new(program1_module_coupon::CouponModule::new(pool.clone()));
    let review_module = Arc::new(program1_module_review::ReviewModule::new(pool.clone()));

    let _ = user_module.seed_default_users().await;
    let _ = catalog_module.seed_default_catalog().await;
    let _ = channel_module.seed_default_channels().await;

    let admin_user = user_module.authenticate("admin", "admin123").await.unwrap();
    let admin_token = auth_module.generate_token(&admin_user).unwrap();

    let buyer_id = Uuid::new_v4();
    let now_str = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO buyer_accounts (id, email, full_name, phone_number, phone_verified, is_active, created_at, updated_at)
         VALUES ($1, 'buyer.chat@test.com', 'Chat Buyer', '+628123456789', 1, 1, $2, $3)",
    )
    .bind(buyer_id.to_string())
    .bind(&now_str)
    .bind(&now_str)
    .execute(&pool)
    .await
    .unwrap();

    let buyer_dto = program1_contracts::BuyerAccountDto {
        id: buyer_id,
        google_sub: Some("sub_chat".to_string()),
        email: "buyer.chat@test.com".to_string(),
        full_name: "Chat Buyer".to_string(),
        avatar_url: None,
        phone_number: Some("+628123456789".to_string()),
        phone_verified: true,
        is_active: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let buyer_token = auth_module.generate_buyer_token(&buyer_dto).unwrap();

    let state = AppState {
        store_name: "Test Store".to_string(),
        store_currency: "IDR".to_string(),
        store_whatsapp_number: "085810007735".to_string(),
        user_contract: user_module,
        auth_contract: auth_module,
        catalog_contract: catalog_module,
        inventory_contract: inventory_module,
        channel_contract: channel_module,
        order_contract: order_module,
        analytics_contract: analytics_module,
        audit_contract: audit_module,
        buyer_contract: buyer_module,
        chat_contract: chat_module,
        payment_contract: payment_module,
        coupon_contract: coupon_module,
        review_contract: review_module,
        shipping_contract: Arc::new(program1_module_shipping::ShippingModule::new("".to_string(), "starter".to_string(), "152".to_string())),
        rate_limiter,
        started_at: std::time::Instant::now(),
        google_client_id: "test".to_string(),
    };

    (create_app(state), admin_token, buyer_token, buyer_id)
}

#[tokio::test]
async fn test_chat_room_creation_and_messaging_flow() {
    let (app, admin_token, buyer_token, buyer_id) = setup_test_app().await;

    // 1. Buyer creates or retrieves chat room
    let create_room_req = Request::builder()
        .method("POST")
        .uri("/api/v1/chat/rooms")
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(create_room_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let room: ChatRoomDto = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(room.buyer_id, buyer_id);
    assert_eq!(room.unread_count, 0);
    assert!(room.last_message.is_none());

    // 2. Buyer sends first message
    let send_msg_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/chat/rooms/{}/messages", room.id))
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "content": "Halo admin, ready stok kak?"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(send_msg_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let body_bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let msg: ChatMessageDto = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(msg.room_id, room.id);
    assert_eq!(msg.sender_type, ChatSenderType::Buyer);
    assert_eq!(msg.content, "Halo admin, ready stok kak?");

    // 3. Admin lists chat rooms and sees 1 active room with unread_count = 1
    let list_rooms_req = Request::builder()
        .method("GET")
        .uri("/api/v1/admin/chat/rooms")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(list_rooms_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let rooms: Vec<ChatRoomDto> = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(rooms.len(), 1);
    assert_eq!(rooms[0].id, room.id);
    assert_eq!(rooms[0].unread_count, 1);
    assert_eq!(
        rooms[0].last_message.as_deref(),
        Some("Halo admin, ready stok kak?")
    );

    // 4. Admin replies to buyer
    let admin_reply_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/admin/chat/rooms/{}/messages", room.id))
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "content": "Halo, barang ready banyak ya kak!"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(admin_reply_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let body_bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let reply_msg: ChatMessageDto = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(reply_msg.sender_type, ChatSenderType::Seller);
    assert_eq!(reply_msg.content, "Halo, barang ready banyak ya kak!");

    // 5. Admin lists chat rooms again -> unread_count is reset to 0
    let list_rooms_req2 = Request::builder()
        .method("GET")
        .uri("/api/v1/admin/chat/rooms")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(list_rooms_req2).await.unwrap();
    let body_bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let rooms2: Vec<ChatRoomDto> = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(rooms2[0].unread_count, 0);
    assert_eq!(
        rooms2[0].last_message.as_deref(),
        Some("Halo, barang ready banyak ya kak!")
    );

    // 6. Buyer retrieves full message thread
    let get_messages_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/chat/rooms/{}/messages?limit=50", room.id))
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(get_messages_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let messages: Vec<ChatMessageDto> = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].sender_type, ChatSenderType::Buyer);
    assert_eq!(messages[0].content, "Halo admin, ready stok kak?");
    assert_eq!(messages[1].sender_type, ChatSenderType::Seller);
    assert_eq!(messages[1].content, "Halo, barang ready banyak ya kak!");
}

#[tokio::test]
async fn test_chat_unauthorized_and_validation_errors() {
    let (app, admin_token, buyer_token, _buyer_id) = setup_test_app().await;

    // 1. Anonymous calling buyer room endpoint -> 401
    let anon_req = Request::builder()
        .method("POST")
        .uri("/api/v1/chat/rooms")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(anon_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 2. Anonymous calling admin chat endpoint -> 401
    let anon_admin_req = Request::builder()
        .method("GET")
        .uri("/api/v1/admin/chat/rooms")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(anon_admin_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 3. Buyer attempting to access seller admin chat endpoint -> 401/403
    let buyer_on_admin_req = Request::builder()
        .method("GET")
        .uri("/api/v1/admin/chat/rooms")
        .header(header::AUTHORIZATION, format!("Bearer {}", buyer_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(buyer_on_admin_req).await.unwrap();
    assert!(res.status() == StatusCode::UNAUTHORIZED || res.status() == StatusCode::FORBIDDEN);

    // 4. Validation error: Empty message content -> 422
    let dummy_room_id = Uuid::new_v4();
    let empty_msg_req = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/v1/admin/chat/rooms/{}/messages",
            dummy_room_id
        ))
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "content": "" }).to_string()))
        .unwrap();

    let res = app.clone().oneshot(empty_msg_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body_bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json_res: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json_res["error"]["code"], "VALIDATION_FAILED");
}
