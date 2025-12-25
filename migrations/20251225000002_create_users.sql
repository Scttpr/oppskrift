-- Users table
-- Migration: 20251225000002_create_users

-- Function to auto-update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(50) NOT NULL,
    display_name VARCHAR(100) NOT NULL,
    bio TEXT,
    avatar_url VARCHAR(2048),
    measurement_pref measurement_pref NOT NULL DEFAULT 'metric',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ap_id VARCHAR(2048) NOT NULL,

    CONSTRAINT users_username_unique UNIQUE (username),
    CONSTRAINT users_ap_id_unique UNIQUE (ap_id)
);

-- Indexes
CREATE INDEX idx_users_username ON users (username);
CREATE INDEX idx_users_ap_id ON users (ap_id);

-- Trigger for auto-updating updated_at
CREATE TRIGGER set_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();
