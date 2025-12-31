-- Create group_members table for user-group associations
-- Members of a group inherit permissions granted to that group

CREATE TABLE group_members (
    group_id UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    added_by UUID REFERENCES users(id) ON DELETE SET NULL,
    PRIMARY KEY (group_id, user_id)
);

-- Index for finding what groups a user belongs to
CREATE INDEX idx_group_members_user ON group_members(user_id);
