-- Migration: 009_buyer_email_password_auth.sql

-- 1. Create modernized buyer accounts table with nullable google_sub and password_hash
CREATE TABLE IF NOT EXISTS buyer_accounts_v2 (
    id TEXT PRIMARY KEY,
    google_sub TEXT UNIQUE,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT,
    full_name TEXT NOT NULL,
    avatar_url TEXT,
    phone_number TEXT,
    phone_verified INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- 2. Migrate existing records from buyer_accounts
INSERT OR IGNORE INTO buyer_accounts_v2 (id, google_sub, email, password_hash, full_name, avatar_url, phone_number, phone_verified, is_active, created_at, updated_at)
SELECT id, google_sub, email, NULL, full_name, avatar_url, phone_number, phone_verified, is_active, created_at, updated_at
FROM buyer_accounts;

-- 3. Replace old table with new schema
DROP TABLE buyer_accounts;
ALTER TABLE buyer_accounts_v2 RENAME TO buyer_accounts;

-- 4. Recreate indices
CREATE INDEX IF NOT EXISTS idx_buyer_accounts_google_sub ON buyer_accounts(google_sub);
CREATE INDEX IF NOT EXISTS idx_buyer_accounts_email ON buyer_accounts(email);
