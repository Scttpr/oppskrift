-- Pending 2FA verification tokens
-- Migration: 20251227000001_two_factor_pending
-- Feature: 002-user-auth (2FA login flow completion)

-- Table for temporary tokens during 2FA login flow
CREATE TABLE IF NOT EXISTS two_factor_pending_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(64) NOT NULL,  -- SHA-256 hash
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,

    CONSTRAINT two_factor_pending_tokens_unique_user UNIQUE (user_id)
);

-- Index for token lookup
CREATE INDEX IF NOT EXISTS idx_two_factor_pending_token_hash ON two_factor_pending_tokens (token_hash);

-- Index for cleanup
CREATE INDEX IF NOT EXISTS idx_two_factor_pending_expires ON two_factor_pending_tokens (expires_at);

-- Comment
COMMENT ON TABLE two_factor_pending_tokens IS 'Short-lived tokens for pending 2FA verification during login';
