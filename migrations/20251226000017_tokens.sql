-- Password reset and email confirmation tokens
-- Migration: 20251226000017_tokens
-- Feature: 002-user-auth

-- Password reset tokens
CREATE TABLE password_reset_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- SHA-256 hash of the token (token itself sent via email)
    token_hash VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    -- Single-use: set when token is consumed
    used_at TIMESTAMPTZ,

    CONSTRAINT password_reset_token_hash_unique UNIQUE (token_hash)
);

-- Email confirmation tokens
CREATE TABLE email_confirmation_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- user_id can be NULL for new registrations before user is created
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    -- Email being confirmed (may differ from user's current email for change requests)
    email VARCHAR(255) NOT NULL,
    -- SHA-256 hash of the token
    token_hash VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,

    CONSTRAINT email_confirmation_token_hash_unique UNIQUE (token_hash)
);

-- Indexes for token lookups
CREATE INDEX idx_password_reset_tokens_user_id ON password_reset_tokens (user_id);
CREATE INDEX idx_password_reset_tokens_hash ON password_reset_tokens (token_hash);
CREATE INDEX idx_password_reset_tokens_expires ON password_reset_tokens (expires_at);

CREATE INDEX idx_email_confirmation_tokens_user_id ON email_confirmation_tokens (user_id);
CREATE INDEX idx_email_confirmation_tokens_hash ON email_confirmation_tokens (token_hash);
CREATE INDEX idx_email_confirmation_tokens_email ON email_confirmation_tokens (email);
CREATE INDEX idx_email_confirmation_tokens_expires ON email_confirmation_tokens (expires_at);

COMMENT ON TABLE password_reset_tokens IS 'One-time password reset tokens (1h expiry)';
COMMENT ON TABLE email_confirmation_tokens IS 'Email confirmation tokens (24h expiry)';
