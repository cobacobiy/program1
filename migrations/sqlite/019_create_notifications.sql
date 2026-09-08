-- 019_create_notifications.sql
-- In-app notification center for buyers and sellers

CREATE TABLE IF NOT EXISTS notifications (
    id TEXT PRIMARY KEY NOT NULL,
    recipient_type TEXT NOT NULL,     -- 'buyer' or 'seller'
    recipient_id TEXT NOT NULL,       -- buyer_id or user_id (or 'admin')
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    notification_type TEXT NOT NULL,  -- 'order_status', 'new_order', 'low_stock', 'chat', 'promo', 'system'
    reference_id TEXT,               -- order_id, product_id, chat_room_id, etc.
    reference_type TEXT,             -- 'order', 'product', 'chat', etc.
    is_read INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_notifications_recipient ON notifications(recipient_type, recipient_id, is_read);
CREATE INDEX IF NOT EXISTS idx_notifications_created ON notifications(created_at DESC);
