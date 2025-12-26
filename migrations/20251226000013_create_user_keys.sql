-- User cryptographic keys for ActivityPub federation
-- Each user has a RSA keypair for HTTP Signatures

CREATE TABLE user_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    public_key_pem TEXT NOT NULL,
    private_key_pem TEXT NOT NULL,
    algorithm VARCHAR(50) NOT NULL DEFAULT 'RSA-SHA256',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    rotated_at TIMESTAMPTZ,
    UNIQUE(user_id)
);

-- Index for fast lookup by user_id
CREATE INDEX idx_user_keys_user_id ON user_keys(user_id);

-- Comment for documentation
COMMENT ON TABLE user_keys IS 'RSA keypairs for ActivityPub HTTP Signatures';
COMMENT ON COLUMN user_keys.public_key_pem IS 'PEM-encoded RSA public key';
COMMENT ON COLUMN user_keys.private_key_pem IS 'PEM-encoded RSA private key';
COMMENT ON COLUMN user_keys.algorithm IS 'Signature algorithm (default: RSA-SHA256)';
COMMENT ON COLUMN user_keys.rotated_at IS 'Last key rotation timestamp';
