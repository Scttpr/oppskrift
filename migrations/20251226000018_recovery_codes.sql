-- 2FA Recovery codes
-- Migration: 20251226000018_recovery_codes
-- Feature: 002-user-auth

CREATE TABLE recovery_codes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Bcrypt hash of the recovery code (8 codes per user, 8 chars each)
    code_hash VARCHAR(72) NOT NULL,
    -- Single-use: set when code is consumed
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for user's recovery codes
CREATE INDEX idx_recovery_codes_user_id ON recovery_codes (user_id);

-- Index for finding unused codes
CREATE INDEX idx_recovery_codes_unused ON recovery_codes (user_id, used_at)
    WHERE used_at IS NULL;

COMMENT ON TABLE recovery_codes IS 'Single-use 2FA recovery codes (8 per user)';
COMMENT ON COLUMN recovery_codes.code_hash IS 'Bcrypt hash - codes shown once at 2FA setup';
