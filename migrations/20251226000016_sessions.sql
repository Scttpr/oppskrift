-- Sessions table for auth
-- Migration: 20251226000016_sessions
-- Feature: 002-user-auth

CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- SHA-256 hash of the session token (token itself never stored)
    token_hash VARCHAR(64) NOT NULL,
    -- Device/client info for session management UI
    device_info VARCHAR(255),
    ip_address INET,
    user_agent TEXT,
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_activity TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,

    CONSTRAINT sessions_token_hash_unique UNIQUE (token_hash)
);

-- Indexes for session lookups
CREATE INDEX idx_sessions_user_id ON sessions (user_id);
CREATE INDEX idx_sessions_token_hash ON sessions (token_hash);
CREATE INDEX idx_sessions_expires_at ON sessions (expires_at);

-- No partial index for expired sessions - use expires_at index for cleanup queries
-- (partial indexes require IMMUTABLE predicates, NOW() is not)

COMMENT ON TABLE sessions IS 'User authentication sessions with secure token storage';
COMMENT ON COLUMN sessions.token_hash IS 'SHA-256 hash of session token - never store plaintext';
