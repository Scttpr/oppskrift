-- Add federation_enabled field to users table
-- Allows users to opt-out of ActivityPub federation

ALTER TABLE users
ADD COLUMN federation_enabled BOOLEAN NOT NULL DEFAULT true;

-- Index for filtering federated users
CREATE INDEX idx_users_federation_enabled ON users(federation_enabled) WHERE federation_enabled = true;

-- Comment for documentation
COMMENT ON COLUMN users.federation_enabled IS 'Whether user participates in ActivityPub federation';
