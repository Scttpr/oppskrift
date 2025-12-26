-- Security events audit log
-- Migration: 20251226000019_security_events
-- Feature: 002-user-auth

-- Enum for security event types
CREATE TYPE security_event_type AS ENUM (
    -- Registration
    'register_success',
    'register_failure',
    -- Login/Logout
    'login_success',
    'login_failure',
    'login_locked',
    'logout',
    -- Password
    'password_reset_request',
    'password_reset_complete',
    'password_change',
    -- Email
    'email_change',
    'email_confirmed',
    -- 2FA
    'totp_enable',
    'totp_disable',
    'recovery_code_used',
    -- Sessions
    'session_revoke',
    'session_revoke_all',
    -- Account deletion (GDPR)
    'account_delete_request',
    'account_delete_cancel',
    'account_delete_execute',
    -- Security alerts
    'rate_limit_exceeded',
    'suspicious_activity'
);

CREATE TABLE security_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- user_id can be NULL for failed auth attempts on non-existent accounts
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    event_type security_event_type NOT NULL,
    ip_address INET,
    user_agent TEXT,
    -- Additional context as JSON (e.g., old email domain, sessions_revoked count)
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for security event queries
CREATE INDEX idx_security_events_user_id ON security_events (user_id);
CREATE INDEX idx_security_events_type ON security_events (event_type);
CREATE INDEX idx_security_events_created_at ON security_events (created_at);
CREATE INDEX idx_security_events_ip ON security_events (ip_address);

-- Composite index for user's recent events
CREATE INDEX idx_security_events_user_recent ON security_events (user_id, created_at DESC);

-- Index for cleanup of old events (GDPR: 90 days for session events, longer for others)
CREATE INDEX idx_security_events_cleanup ON security_events (event_type, created_at);

COMMENT ON TABLE security_events IS 'Audit log for all security-relevant actions';
COMMENT ON COLUMN security_events.metadata IS 'Additional context without PII (use email domain, not full email)';
