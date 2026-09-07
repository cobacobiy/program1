use async_trait::async_trait;
use chrono::{DateTime, Utc};
use program1_contracts::{
    ChatContract, ChatMessageDto, ChatRoomDto, ChatSenderType, ContractError,
};
use program1_core::database::DbPool;
use sqlx::Row;
use uuid::Uuid;

#[derive(Clone)]
pub struct ChatModule {
    pool: DbPool,
}

impl ChatModule {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    fn parse_datetime(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|_| {
                chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
            })
            .unwrap_or_else(|_| Utc::now())
    }

    fn row_to_room(row: &sqlx::sqlite::SqliteRow) -> Result<ChatRoomDto, ContractError> {
        let id_str: String = row.get("id");
        let id = Uuid::parse_str(&id_str)
            .map_err(|e| ContractError::Internal(format!("Corrupt room UUID: {}", e)))?;

        let buyer_id_str: String = row.get("buyer_id");
        let buyer_id = Uuid::parse_str(&buyer_id_str)
            .map_err(|e| ContractError::Internal(format!("Corrupt buyer UUID: {}", e)))?;

        let buyer_name: String = row.get("buyer_name");
        let last_message: Option<String> = row.get("last_message");
        let unread_count: i64 = row.get("unread_count");

        let updated_at_str: String = row.get("updated_at");
        let created_at_str: String = row.get("created_at");

        Ok(ChatRoomDto {
            id,
            buyer_id,
            buyer_name,
            last_message,
            unread_count,
            updated_at: Self::parse_datetime(&updated_at_str),
            created_at: Self::parse_datetime(&created_at_str),
        })
    }

    fn row_to_message(row: &sqlx::sqlite::SqliteRow) -> Result<ChatMessageDto, ContractError> {
        let id_str: String = row.get("id");
        let id = Uuid::parse_str(&id_str)
            .map_err(|e| ContractError::Internal(format!("Corrupt message UUID: {}", e)))?;

        let room_id_str: String = row.get("room_id");
        let room_id = Uuid::parse_str(&room_id_str)
            .map_err(|e| ContractError::Internal(format!("Corrupt room UUID: {}", e)))?;

        let sender_type_str: String = row.get("sender_type");
        let sender_type = match sender_type_str.as_str() {
            "Buyer" => ChatSenderType::Buyer,
            "Seller" => ChatSenderType::Seller,
            "System" => ChatSenderType::System,
            other => {
                return Err(ContractError::Internal(format!(
                    "Unknown sender_type in DB: {}",
                    other
                )));
            }
        };

        let sender_id_str: String = row.get("sender_id");
        let sender_id = Uuid::parse_str(&sender_id_str)
            .map_err(|e| ContractError::Internal(format!("Corrupt sender UUID: {}", e)))?;

        let sender_name: String = row.get("sender_name");
        let content: String = row.get("content");
        let is_read_int: i64 = row.get("is_read");
        let is_read = is_read_int != 0;

        let created_at_str: String = row.get("created_at");

        Ok(ChatMessageDto {
            id,
            room_id,
            sender_type,
            sender_id,
            sender_name,
            content,
            is_read,
            created_at: Self::parse_datetime(&created_at_str),
        })
    }
}

fn sender_type_to_str(st: ChatSenderType) -> &'static str {
    match st {
        ChatSenderType::Buyer => "Buyer",
        ChatSenderType::Seller => "Seller",
        ChatSenderType::System => "System",
    }
}

#[async_trait]
impl ChatContract for ChatModule {
    async fn send_message(
        &self,
        room_id: Uuid,
        sender_type: ChatSenderType,
        sender_id: Uuid,
        sender_name: &str,
        content: &str,
    ) -> Result<ChatMessageDto, ContractError> {
        let clean_content = content.trim();
        if clean_content.is_empty() {
            return Err(ContractError::ValidationError(
                "Pesan obrolan tidak boleh kosong".to_string(),
            ));
        }

        let message_id = Uuid::new_v4();
        let room_id_str = room_id.to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let sender_type_str = sender_type_to_str(sender_type);

        // Verify room exists
        let room_exists = sqlx::query("SELECT id FROM chat_rooms WHERE id = $1")
            .bind(&room_id_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ContractError::Internal(format!("Failed to verify chat room: {}", e)))?;

        if room_exists.is_none() {
            return Err(ContractError::NotFound(format!(
                "Chat room {} tidak ditemukan",
                room_id
            )));
        }

        // Insert message
        sqlx::query(
            "INSERT INTO chat_messages (id, room_id, sender_type, sender_id, sender_name, content, is_read, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, 0, $7)",
        )
        .bind(message_id.to_string())
        .bind(&room_id_str)
        .bind(sender_type_str)
        .bind(sender_id.to_string())
        .bind(sender_name)
        .bind(clean_content)
        .bind(&now_str)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(format!("Failed to save chat message: {}", e)))?;

        // Update room status
        match sender_type {
            ChatSenderType::Buyer => {
                sqlx::query(
                    "UPDATE chat_rooms
                     SET last_message = $1, unread_count = unread_count + 1, updated_at = $2
                     WHERE id = $3",
                )
                .bind(clean_content)
                .bind(&now_str)
                .bind(&room_id_str)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    ContractError::Internal(format!("Failed to update chat room: {}", e))
                })?;
            }
            ChatSenderType::Seller => {
                sqlx::query(
                    "UPDATE chat_rooms
                     SET last_message = $1, unread_count = 0, updated_at = $2
                     WHERE id = $3",
                )
                .bind(clean_content)
                .bind(&now_str)
                .bind(&room_id_str)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    ContractError::Internal(format!("Failed to update chat room: {}", e))
                })?;

                // Mark buyer messages as read
                let _ = sqlx::query(
                    "UPDATE chat_messages SET is_read = 1 WHERE room_id = $1 AND sender_type = 'Buyer'",
                )
                .bind(&room_id_str)
                .execute(&self.pool)
                .await;
            }
            ChatSenderType::System => {
                sqlx::query(
                    "UPDATE chat_rooms
                     SET last_message = $1, updated_at = $2
                     WHERE id = $3",
                )
                .bind(clean_content)
                .bind(&now_str)
                .bind(&room_id_str)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    ContractError::Internal(format!("Failed to update chat room: {}", e))
                })?;
            }
        }

        Ok(ChatMessageDto {
            id: message_id,
            room_id,
            sender_type,
            sender_id,
            sender_name: sender_name.to_string(),
            content: clean_content.to_string(),
            is_read: false,
            created_at: now,
        })
    }

    async fn get_messages(
        &self,
        room_id: Uuid,
        limit: i64,
    ) -> Result<Vec<ChatMessageDto>, ContractError> {
        let safe_limit = limit.clamp(1, 200);
        let room_id_str = room_id.to_string();

        let rows = sqlx::query(
            "SELECT id, room_id, sender_type, sender_id, sender_name, content, is_read, created_at
             FROM chat_messages
             WHERE room_id = $1
             ORDER BY created_at DESC
             LIMIT $2",
        )
        .bind(&room_id_str)
        .bind(safe_limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(format!("Failed to fetch chat messages: {}", e)))?;

        let mut messages = Vec::with_capacity(rows.len());
        for r in &rows {
            messages.push(Self::row_to_message(r)?);
        }

        // Return in chronological order (oldest to newest)
        messages.reverse();

        Ok(messages)
    }

    async fn get_or_create_buyer_room(
        &self,
        buyer_id: Uuid,
        buyer_name: &str,
    ) -> Result<ChatRoomDto, ContractError> {
        let buyer_id_str = buyer_id.to_string();

        let existing = sqlx::query(
            "SELECT id, buyer_id, buyer_name, last_message, unread_count, updated_at, created_at
             FROM chat_rooms
             WHERE buyer_id = $1",
        )
        .bind(&buyer_id_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(format!("Failed to query chat room: {}", e)))?;

        if let Some(row) = existing {
            return Self::row_to_room(&row);
        }

        let room_id = Uuid::new_v4();
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        sqlx::query(
            "INSERT INTO chat_rooms (id, buyer_id, buyer_name, last_message, unread_count, updated_at, created_at)
             VALUES ($1, $2, $3, NULL, 0, $4, $5)
             ON CONFLICT(buyer_id) DO UPDATE SET buyer_name = excluded.buyer_name",
        )
        .bind(room_id.to_string())
        .bind(&buyer_id_str)
        .bind(buyer_name)
        .bind(&now_str)
        .bind(&now_str)
        .execute(&self.pool)
        .await
        .map_err(|e| ContractError::Internal(format!("Failed to create chat room: {}", e)))?;

        let row = sqlx::query(
            "SELECT id, buyer_id, buyer_name, last_message, unread_count, updated_at, created_at
             FROM chat_rooms
             WHERE buyer_id = $1",
        )
        .bind(&buyer_id_str)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            ContractError::Internal(format!("Failed to retrieve created chat room: {}", e))
        })?;

        Self::row_to_room(&row)
    }

    async fn list_active_rooms(&self) -> Result<Vec<ChatRoomDto>, ContractError> {
        let rows = sqlx::query(
            "SELECT id, buyer_id, buyer_name, last_message, unread_count, updated_at, created_at
             FROM chat_rooms
             ORDER BY updated_at DESC
             LIMIT 100",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            ContractError::Internal(format!("Failed to fetch active chat rooms: {}", e))
        })?;

        let mut rooms = Vec::with_capacity(rows.len());
        for r in &rows {
            rooms.push(Self::row_to_room(r)?);
        }

        Ok(rooms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use program1_core::init_database;

    #[tokio::test]
    async fn test_get_or_create_buyer_room_is_idempotent() {
        let pool = init_database("sqlite::memory:").await.unwrap();
        let module = ChatModule::new(pool);

        let buyer_id = Uuid::new_v4();
        let room1 = module
            .get_or_create_buyer_room(buyer_id, "Budi Santoso")
            .await
            .unwrap();
        assert_eq!(room1.buyer_id, buyer_id);
        assert_eq!(room1.buyer_name, "Budi Santoso");
        assert_eq!(room1.unread_count, 0);
        assert!(room1.last_message.is_none());

        // Call again with same buyer_id
        let room2 = module
            .get_or_create_buyer_room(buyer_id, "Budi Santoso")
            .await
            .unwrap();
        assert_eq!(room1.id, room2.id);
        assert_eq!(room2.buyer_id, buyer_id);
    }

    #[tokio::test]
    async fn test_send_and_retrieve_messages() {
        let pool = init_database("sqlite::memory:").await.unwrap();
        let module = ChatModule::new(pool);

        let buyer_id = Uuid::new_v4();
        let seller_id = Uuid::new_v4();
        let room = module
            .get_or_create_buyer_room(buyer_id, "Ani")
            .await
            .unwrap();

        // 1. Buyer sends message
        let msg1 = module
            .send_message(
                room.id,
                ChatSenderType::Buyer,
                buyer_id,
                "Ani",
                "Halo, apakah stok barang ini ready?",
            )
            .await
            .unwrap();
        assert_eq!(msg1.room_id, room.id);
        assert_eq!(msg1.sender_type, ChatSenderType::Buyer);
        assert_eq!(msg1.content, "Halo, apakah stok barang ini ready?");

        // Verify room unread_count incremented
        let active_rooms = module.list_active_rooms().await.unwrap();
        assert_eq!(active_rooms.len(), 1);
        assert_eq!(active_rooms[0].unread_count, 1);
        assert_eq!(
            active_rooms[0].last_message.as_deref(),
            Some("Halo, apakah stok barang ini ready?")
        );

        // 2. Seller replies
        let msg2 = module
            .send_message(
                room.id,
                ChatSenderType::Seller,
                seller_id,
                "Admin Toko",
                "Ready kak, silakan diorder ya!",
            )
            .await
            .unwrap();
        assert_eq!(msg2.sender_type, ChatSenderType::Seller);

        // Verify room unread_count reset to 0 by seller reply
        let active_rooms_after = module.list_active_rooms().await.unwrap();
        assert_eq!(active_rooms_after[0].unread_count, 0);
        assert_eq!(
            active_rooms_after[0].last_message.as_deref(),
            Some("Ready kak, silakan diorder ya!")
        );

        // 3. Retrieve messages
        let messages = module.get_messages(room.id, 50).await.unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "Halo, apakah stok barang ini ready?");
        assert_eq!(messages[1].content, "Ready kak, silakan diorder ya!");
    }

    #[tokio::test]
    async fn test_messages_limit_and_chronological_order() {
        let pool = init_database("sqlite::memory:").await.unwrap();
        let module = ChatModule::new(pool);

        let buyer_id = Uuid::new_v4();
        let room = module
            .get_or_create_buyer_room(buyer_id, "Chandra")
            .await
            .unwrap();

        for i in 1..=5 {
            module
                .send_message(
                    room.id,
                    ChatSenderType::Buyer,
                    buyer_id,
                    "Chandra",
                    &format!("Pesan ke-{}", i),
                )
                .await
                .unwrap();
        }

        // Fetch with limit 3 (should return latest 3, in chronological order: 3, 4, 5)
        let messages = module.get_messages(room.id, 3).await.unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].content, "Pesan ke-3");
        assert_eq!(messages[1].content, "Pesan ke-4");
        assert_eq!(messages[2].content, "Pesan ke-5");
    }

    #[tokio::test]
    async fn test_send_message_empty_content_validation() {
        let pool = init_database("sqlite::memory:").await.unwrap();
        let module = ChatModule::new(pool);

        let buyer_id = Uuid::new_v4();
        let room = module
            .get_or_create_buyer_room(buyer_id, "Dewi")
            .await
            .unwrap();

        let res = module
            .send_message(room.id, ChatSenderType::Buyer, buyer_id, "Dewi", "    ")
            .await;
        assert!(res.is_err());
        match res {
            Err(ContractError::ValidationError(msg)) => {
                assert!(msg.contains("tidak boleh kosong"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }
}
