-- Create permission system types and table
-- Supports ABAC with user, group, and instance subjects

-- Permission level enum (hierarchy: edit > contributor > view)
CREATE TYPE permission_level AS ENUM ('view', 'edit', 'contributor');

-- Subject type enum
CREATE TYPE subject_type AS ENUM ('user', 'group', 'instance');

-- Permissions table
CREATE TABLE permissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    resource_type TEXT NOT NULL CHECK (resource_type IN ('recipe', 'book')),
    resource_id UUID NOT NULL,
    subject_type subject_type NOT NULL,
    subject_id UUID,
    subject_domain TEXT,
    permission_level permission_level NOT NULL,
    granted_by UUID REFERENCES users(id) ON DELETE SET NULL,
    granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ensure valid subject: user/group needs subject_id, instance needs domain
    CONSTRAINT valid_subject CHECK (
        (subject_type IN ('user', 'group') AND subject_id IS NOT NULL AND subject_domain IS NULL)
        OR
        (subject_type = 'instance' AND subject_id IS NULL AND subject_domain IS NOT NULL)
    ),

    -- Contributor level only valid for books
    CONSTRAINT contributor_only_for_books CHECK (
        permission_level != 'contributor' OR resource_type = 'book'
    ),

    -- Unique constraint for deduplication
    UNIQUE (resource_type, resource_id, subject_type, subject_id, subject_domain)
);

-- Index for finding permissions on a resource
CREATE INDEX idx_permissions_resource ON permissions(resource_type, resource_id);

-- Index for finding user's direct permissions
CREATE INDEX idx_permissions_user ON permissions(subject_type, subject_id)
    WHERE subject_type = 'user';

-- Index for finding group permissions
CREATE INDEX idx_permissions_group ON permissions(subject_type, subject_id)
    WHERE subject_type = 'group';

-- Index for finding instance permissions
CREATE INDEX idx_permissions_instance ON permissions(subject_domain)
    WHERE subject_type = 'instance';
