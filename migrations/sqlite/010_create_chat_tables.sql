-- Migration: 010_create_chat_tables.sql

-- 1. Chat rooms (one dedicated room per buyer)
CREATE TABLE IF NOT EXISTS chat_rooms (
    id TEXT PRIMARY KEY,
    buyer_id TEXT NOT NULL,
    buyer_name TEXT NOT NULL DEFAULT '',
    last_message TEXT,
    unread_count INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(buyer_id)
);

-- 2. Chat messages (chronological conversation thread)
CREATE TABLE IF NOT EXISTS chat_messages (
    id TEXT PRIMARY KEY,
    room_id TEXT NOT NULL REFERENCES chat_rooms(id),
    sender_type TEXT NOT NULL CHECK(sender_type IN ('Buyer', 'Seller', 'System')),
    sender_id TEXT NOT NULL,
    sender_name TEXT NOT NULL DEFAULT '',
    content TEXT NOT NULL,
    is_read INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- 3. Indexes for fast retrieval
CREATE INDEX IF NOT EXISTS idx_chat_messages_room ON chat_messages(room_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_chat_rooms_buyer ON chat_rooms(buyer_id);
CREATE INDEX IF NOT EXISTS idx_chat_rooms_updated ON chat_rooms(updated_at DESC);
