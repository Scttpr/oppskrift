-- Follow relationships between users
CREATE TABLE follows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    follower_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    following_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ap_id VARCHAR(2048) NOT NULL UNIQUE,

    -- Prevent duplicate follows
    CONSTRAINT follows_unique UNIQUE (follower_id, following_id),
    -- Prevent self-follow
    CONSTRAINT follows_no_self CHECK (follower_id != following_id)
);

-- Indexes for efficient queries
CREATE INDEX idx_follows_follower_id ON follows(follower_id);
CREATE INDEX idx_follows_following_id ON follows(following_id);
