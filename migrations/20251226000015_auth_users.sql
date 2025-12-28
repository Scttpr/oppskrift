-- Auth fields for users table
-- Migration: 20251226000015_auth_users
-- Feature: 002-user-auth

-- Add authentication fields to existing users table
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS email VARCHAR(255),
    ADD COLUMN IF NOT EXISTS email_verified BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS password_hash VARCHAR(255),
    ADD COLUMN IF NOT EXISTS totp_secret_encrypted BYTEA,
    ADD COLUMN IF NOT EXISTS totp_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS failed_login_attempts INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS locked_until TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS deletion_requested_at TIMESTAMPTZ;

-- Email must be unique (case-insensitive)
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email_lower ON users (LOWER(email));

-- Index for login lookups
CREATE INDEX IF NOT EXISTS idx_users_email ON users (email);

-- Index for account cleanup jobs
CREATE INDEX IF NOT EXISTS idx_users_deletion_requested ON users (deletion_requested_at)
    WHERE deletion_requested_at IS NOT NULL;

-- Comment on security-sensitive columns
COMMENT ON COLUMN users.password_hash IS 'Argon2id hash of user password';
COMMENT ON COLUMN users.totp_secret_encrypted IS 'AES-256-GCM encrypted TOTP secret';
COMMENT ON COLUMN users.failed_login_attempts IS 'Count of failed login attempts for lockout';
COMMENT ON COLUMN users.locked_until IS 'Account locked until this timestamp';
COMMENT ON COLUMN users.deletion_requested_at IS 'GDPR: deletion grace period start';
