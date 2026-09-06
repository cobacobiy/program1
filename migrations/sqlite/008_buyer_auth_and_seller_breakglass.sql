-- Migration: 008_buyer_auth_and_seller_breakglass.sql

-- 1. Buyer accounts table (Email/Password registration and/or Google OAuth + verified phone)
CREATE TABLE IF NOT EXISTS buyer_accounts (
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

CREATE INDEX IF NOT EXISTS idx_buyer_accounts_google_sub ON buyer_accounts(google_sub);
CREATE INDEX IF NOT EXISTS idx_buyer_accounts_email ON buyer_accounts(email);

-- 2. Buyer shipping addresses table (multiple addresses, one default)
CREATE TABLE IF NOT EXISTS buyer_addresses (
    id TEXT PRIMARY KEY,
    buyer_id TEXT NOT NULL,
    recipient_name TEXT NOT NULL,
    phone_number TEXT NOT NULL,
    street_address TEXT NOT NULL,
    subdistrict TEXT NOT NULL DEFAULT '',
    city TEXT NOT NULL DEFAULT '',
    province TEXT NOT NULL DEFAULT '',
    postal_code TEXT NOT NULL DEFAULT '',
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY(buyer_id) REFERENCES buyer_accounts(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_buyer_addresses_buyer_id ON buyer_addresses(buyer_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_buyer_addresses_one_default ON buyer_addresses(buyer_id) WHERE is_default = 1;

-- 3. Buyer OTP verifications table (hash, TTL, attempts limit)
CREATE TABLE IF NOT EXISTS buyer_otp_verifications (
    id TEXT PRIMARY KEY,
    buyer_id TEXT NOT NULL,
    phone_number TEXT NOT NULL,
    otp_hash TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    expires_at TEXT NOT NULL,
    verified_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY(buyer_id) REFERENCES buyer_accounts(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_buyer_otp_lookup ON buyer_otp_verifications(buyer_id, phone_number);

-- 4. Developer Support Break-Glass session state
CREATE TABLE IF NOT EXISTS break_glass_sessions (
    id TEXT PRIMARY KEY,
    account_username TEXT NOT NULL DEFAULT 'dev_support',
    is_active INTEGER NOT NULL DEFAULT 0,
    active_until TEXT,
    activated_by TEXT,
    reason TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- 5. Add shipping address snapshot and buyer reference columns to orders
ALTER TABLE orders ADD COLUMN buyer_id TEXT;
ALTER TABLE orders ADD COLUMN shipping_recipient_name TEXT NOT NULL DEFAULT '';
ALTER TABLE orders ADD COLUMN shipping_phone_number TEXT NOT NULL DEFAULT '';
ALTER TABLE orders ADD COLUMN shipping_street_address TEXT NOT NULL DEFAULT '';
ALTER TABLE orders ADD COLUMN shipping_subdistrict TEXT NOT NULL DEFAULT '';
ALTER TABLE orders ADD COLUMN shipping_city TEXT NOT NULL DEFAULT '';
ALTER TABLE orders ADD COLUMN shipping_province TEXT NOT NULL DEFAULT '';
ALTER TABLE orders ADD COLUMN shipping_postal_code TEXT NOT NULL DEFAULT '';
ALTER TABLE orders ADD COLUMN shipping_snapshot_json TEXT NOT NULL DEFAULT '';
