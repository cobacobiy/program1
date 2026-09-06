use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::{AppState, ValidatedJson};
use program1_contracts::{
    ChatMessageDto, ChatRoomDto, ChatSenderType, JwtClaims, SendMessageRequest,
};

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetChatMessagesQuery {
    pub limit: Option<i64>,
}

/// Get or create chat room for authenticated buyer
#[utoipa::path(
    post,
    path = "/api/v1/chat/rooms",
    responses(
        (status = 200, description = "Chat room retrieved or created", body = ChatRoomDto),
        (status = 401, description = "Unauthorized - Buyer token required")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Buyer Live Chat"
)]
pub async fn buyer_get_or_create_room_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
) -> Result<Json<ChatRoomDto>, ApiError> {
    let buyer_name = if claims.username.trim().is_empty() {
        "Buyer".to_string()
    } else {
        claims.username
    };

    let room = state
        .chat_contract
        .get_or_create_buyer_room(claims.sub, &buyer_name)
        .await?;

    Ok(Json(room))
}

/// Retrieve messages for a chat room (Buyer)
#[utoipa::path(
    get,
    path = "/api/v1/chat/rooms/{room_id}/messages",
    params(
        ("room_id" = Uuid, Path, description = "Chat room identifier"),
        GetChatMessagesQuery
    ),
    responses(
        (status = 200, description = "List of chat messages", body = Vec<ChatMessageDto>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Room not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Buyer Live Chat"
)]
pub async fn buyer_get_messages_handler(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Query(query): Query<GetChatMessagesQuery>,
) -> Result<Json<Vec<ChatMessageDto>>, ApiError> {
    let limit = query.limit.unwrap_or(50);
    let messages = state.chat_contract.get_messages(room_id, limit).await?;
    Ok(Json(messages))
}

/// Send a new message to chat room (Buyer)
#[utoipa::path(
    post,
    path = "/api/v1/chat/rooms/{room_id}/messages",
    params(
        ("room_id" = Uuid, Path, description = "Chat room identifier")
    ),
    request_body = SendMessageRequest,
    responses(
        (status = 201, description = "Message sent successfully", body = ChatMessageDto),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Room not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Buyer Live Chat"
)]
pub async fn buyer_send_message_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(room_id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<SendMessageRequest>,
) -> Result<(StatusCode, Json<ChatMessageDto>), ApiError> {
    let sender_name = if claims.username.trim().is_empty() {
        "Buyer"
    } else {
        &claims.username
    };

    let msg = state
        .chat_contract
        .send_message(
            room_id,
            ChatSenderType::Buyer,
            claims.sub,
            sender_name,
            &req.content,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(msg)))
}

/// List all active chat rooms for merchant dashboard (Seller)
#[utoipa::path(
    get,
    path = "/api/v1/admin/chat/rooms",
    responses(
        (status = 200, description = "List of active chat rooms", body = Vec<ChatRoomDto>),
        (status = 401, description = "Unauthorized - Seller token required")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Admin Live Chat"
)]
pub async fn admin_list_chat_rooms_handler(
    State(state): State<AppState>,
    Extension(_claims): Extension<JwtClaims>,
) -> Result<Json<Vec<ChatRoomDto>>, ApiError> {
    let rooms = state.chat_contract.list_active_rooms().await?;
    Ok(Json(rooms))
}

/// Retrieve messages for a chat room (Seller)
#[utoipa::path(
    get,
    path = "/api/v1/admin/chat/rooms/{room_id}/messages",
    params(
        ("room_id" = Uuid, Path, description = "Chat room identifier"),
        GetChatMessagesQuery
    ),
    responses(
        (status = 200, description = "List of chat messages", body = Vec<ChatMessageDto>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Room not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Admin Live Chat"
)]
pub async fn admin_get_messages_handler(
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Query(query): Query<GetChatMessagesQuery>,
) -> Result<Json<Vec<ChatMessageDto>>, ApiError> {
    let limit = query.limit.unwrap_or(50);
    let messages = state.chat_contract.get_messages(room_id, limit).await?;
    Ok(Json(messages))
}

/// Reply to buyer chat room (Seller)
#[utoipa::path(
    post,
    path = "/api/v1/admin/chat/rooms/{room_id}/messages",
    params(
        ("room_id" = Uuid, Path, description = "Chat room identifier")
    ),
    request_body = SendMessageRequest,
    responses(
        (status = 201, description = "Seller reply sent successfully", body = ChatMessageDto),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Room not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Admin Live Chat"
)]
pub async fn admin_send_message_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<JwtClaims>,
    Path(room_id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<SendMessageRequest>,
) -> Result<(StatusCode, Json<ChatMessageDto>), ApiError> {
    let sender_name = if claims.username.trim().is_empty() {
        "Admin Toko"
    } else {
        &claims.username
    };

    let msg = state
        .chat_contract
        .send_message(
            room_id,
            ChatSenderType::Seller,
            claims.sub,
            sender_name,
            &req.content,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(msg)))
}
