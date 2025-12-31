# Data Model: ABAC Authorization System

**Feature**: 005-abac-authorization
**Date**: 2025-12-30

## Overview

This document defines the data entities, relationships, and validation rules for the ABAC authorization system.

---

## Entities

### 1. Visibility (Enum Extension)

Extends the existing `visibility_type` PostgreSQL enum.

| Value | Description |
|-------|-------------|
| `public` | Accessible to anyone, including unauthenticated users |
| `private` | Accessible only to owner and explicit shares |
| `followers_only` | Accessible to owner and users who follow the owner |

**Migration**: `ALTER TYPE visibility_type ADD VALUE 'followers_only';`

**Affected Models**: `Recipe`, `RecipeBook`

---

### 2. PermissionLevel (New Enum)

Defines the level of access granted.

| Value | Description | Applicable To |
|-------|-------------|---------------|
| `view` | Read-only access | Recipes, Books |
| `edit` | Read and modify | Recipes, Books |
| `contributor` | Add own recipes to book | Books only |

**Hierarchy**: `edit > contributor > view`

When a user has multiple permission paths, the highest level applies.

---

### 3. SubjectType (New Enum)

Defines who can receive permissions.

| Value | Description |
|-------|-------------|
| `user` | A specific user (local or federated) |
| `group` | A group of users |
| `instance` | All users from a federated instance |

---

### 4. Permission (New Entity)

Represents a granted permission on a resource.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `id` | UUID | PK, auto-generated | Unique identifier |
| `resource_type` | TEXT | NOT NULL, CHECK | 'recipe' or 'book' |
| `resource_id` | UUID | NOT NULL, FK | Reference to recipe/book |
| `subject_type` | SubjectType | NOT NULL | Who has the permission |
| `subject_id` | UUID | NULL for instance | User or group ID |
| `subject_domain` | TEXT | For instance type | Domain of federated instance |
| `permission_level` | PermissionLevel | NOT NULL | view, edit, or contributor |
| `granted_by` | UUID | FK to users | Who granted this permission |
| `granted_at` | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | When granted |

**Unique Constraint**: `(resource_type, resource_id, subject_type, subject_id, subject_domain)`

**Indexes**:
- `idx_permission_resource` on `(resource_type, resource_id)`
- `idx_permission_subject` on `(subject_type, subject_id)`
- `idx_permission_instance` on `(subject_type, subject_domain)` WHERE `subject_type = 'instance'`

**Validation Rules**:
- If `subject_type = 'user'` or `subject_type = 'group'`, then `subject_id` is required
- If `subject_type = 'instance'`, then `subject_domain` is required and `subject_id` is NULL
- `contributor` permission is only valid for `resource_type = 'book'`

---

### 5. Group (New Entity)

A named collection of users for batch permission management.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `id` | UUID | PK, auto-generated | Unique identifier |
| `owner_id` | UUID | NOT NULL, FK to users | Creator/owner of group |
| `name` | VARCHAR(100) | NOT NULL | Group name |
| `description` | VARCHAR(500) | NULL | Optional description |
| `created_at` | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | Creation time |
| `updated_at` | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | Last update time |

**Indexes**:
- `idx_group_owner` on `(owner_id)`

**Validation Rules**:
- `name` length: 1-100 characters
- `description` length: 0-500 characters
- Only owner can delete the group

---

### 6. GroupMember (New Entity)

Association between users and groups.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `group_id` | UUID | PK, FK to groups | The group |
| `user_id` | UUID | PK, FK to users | The member |
| `added_at` | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | When added |
| `added_by` | UUID | FK to users | Who added this member |

**Primary Key**: `(group_id, user_id)`

**Indexes**:
- `idx_group_member_user` on `(user_id)` for "what groups is user in" queries

**Cascade**: DELETE on group removes all memberships

---

### 7. BookContribution (New Entity)

Tracks which recipes were added to a book by contributors.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `id` | UUID | PK, auto-generated | Unique identifier |
| `book_id` | UUID | NOT NULL, FK to recipe_books | The book |
| `recipe_id` | UUID | NOT NULL, FK to recipes | The contributed recipe |
| `contributor_id` | UUID | NOT NULL, FK to users | Who added the recipe |
| `added_at` | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | When added |

**Unique Constraint**: `(book_id, recipe_id)`

**Validation Rules**:
- Contributor must own the recipe they're adding
- Contributor must have `contributor` permission on the book

**Cascade**:
- DELETE on recipe removes contribution record
- DELETE on book removes contribution record
- Recipe is NOT deleted when contribution is removed (recipe still exists for owner)

---

### 8. PermissionAuditLog (New Entity)

Immutable log of permission changes for security auditing.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `id` | UUID | PK, auto-generated | Unique identifier |
| `timestamp` | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | When event occurred |
| `event_type` | TEXT | NOT NULL | Event type (see below) |
| `actor_id` | UUID | FK to users, nullable | Who performed the action |
| `resource_type` | TEXT | NULL | Affected resource type |
| `resource_id` | UUID | NULL | Affected resource ID |
| `subject_type` | TEXT | NULL | Permission subject type |
| `subject_id` | UUID | NULL | Permission subject ID |
| `permission_level` | TEXT | NULL | Permission level involved |
| `details` | JSONB | NOT NULL, DEFAULT '{}' | Additional context |

**Event Types**:
- `permission_granted`
- `permission_revoked`
- `group_created`
- `group_deleted`
- `member_added`
- `member_removed`
- `visibility_changed`
- `access_denied`

**Partitioning**: By `timestamp` (monthly partitions)

**Indexes**:
- `idx_audit_actor` on `(actor_id, timestamp DESC)`
- `idx_audit_resource` on `(resource_type, resource_id, timestamp DESC)`

---

## Entity Relationships

```
┌──────────────┐
│    users     │
└──────┬───────┘
       │
       │ 1:N
       ▼
┌──────────────┐         ┌──────────────┐
│    groups    │◄───────►│ group_members│
└──────────────┘   N:M   └──────────────┘
       │
       │ can be subject
       ▼
┌──────────────────────┐
│     permissions      │
│  (resource ↔ subject)│
└──────────┬───────────┘
           │ references
           ▼
┌──────────────┐    ┌──────────────┐
│   recipes    │    │ recipe_books │
└──────────────┘    └──────┬───────┘
                           │
                           │ 1:N
                           ▼
                   ┌──────────────────┐
                   │ book_contributions│
                   │ (tracks who added)│
                   └──────────────────┘
```

---

## State Transitions

### Permission Lifecycle

```
[None] ──grant──► [Active] ──revoke──► [None]
                     │
                     └── (logged to PermissionAuditLog)
```

### Group Membership Lifecycle

```
[Not Member] ──add──► [Member] ──remove──► [Not Member]
                         │
                         └── (invalidates group-based permissions)
```

### Book Contribution Lifecycle

```
[Not in Book] ──add (by contributor)──► [In Book]
                                            │
                    ┌───────────────────────┴───────────────────────┐
                    │                                               │
              ──remove by owner──►                          ──remove by contributor──►
                    │                                               │
                    ▼                                               ▼
              [Not in Book]                                   [Not in Book]
              (recipe still exists)                           (recipe still exists)
```

---

## Existing Entity Modifications

### Recipe

| Field | Change |
|-------|--------|
| `visibility` | Now supports `followers_only` value |

### RecipeBook

| Field | Change |
|-------|--------|
| `visibility` | Now supports `followers_only` value |

---

## Migration Scripts

### Migration 1: Add followers_only visibility

```sql
ALTER TYPE visibility_type ADD VALUE IF NOT EXISTS 'followers_only';
```

### Migration 2: Create groups table

```sql
CREATE TABLE groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    description VARCHAR(500),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_group_owner ON groups(owner_id);
```

### Migration 3: Create group_members table

```sql
CREATE TABLE group_members (
    group_id UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    added_by UUID REFERENCES users(id),
    PRIMARY KEY (group_id, user_id)
);

CREATE INDEX idx_group_member_user ON group_members(user_id);
```

### Migration 4: Create permissions table

```sql
CREATE TYPE permission_level AS ENUM ('view', 'edit', 'contributor');
CREATE TYPE subject_type AS ENUM ('user', 'group', 'instance');

CREATE TABLE permissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    resource_type TEXT NOT NULL CHECK (resource_type IN ('recipe', 'book')),
    resource_id UUID NOT NULL,
    subject_type subject_type NOT NULL,
    subject_id UUID,
    subject_domain TEXT,
    permission_level permission_level NOT NULL,
    granted_by UUID REFERENCES users(id),
    granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_subject CHECK (
        (subject_type IN ('user', 'group') AND subject_id IS NOT NULL AND subject_domain IS NULL)
        OR
        (subject_type = 'instance' AND subject_id IS NULL AND subject_domain IS NOT NULL)
    ),
    CONSTRAINT contributor_only_for_books CHECK (
        permission_level != 'contributor' OR resource_type = 'book'
    ),
    UNIQUE (resource_type, resource_id, subject_type, subject_id, subject_domain)
);

CREATE INDEX idx_permission_resource ON permissions(resource_type, resource_id);
CREATE INDEX idx_permission_user ON permissions(subject_type, subject_id) WHERE subject_type = 'user';
CREATE INDEX idx_permission_group ON permissions(subject_type, subject_id) WHERE subject_type = 'group';
CREATE INDEX idx_permission_instance ON permissions(subject_domain) WHERE subject_type = 'instance';
```

### Migration 5: Create book_contributions table

```sql
CREATE TABLE book_contributions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    book_id UUID NOT NULL REFERENCES recipe_books(id) ON DELETE CASCADE,
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    contributor_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (book_id, recipe_id)
);

CREATE INDEX idx_contribution_book ON book_contributions(book_id);
CREATE INDEX idx_contribution_contributor ON book_contributions(contributor_id);
```

### Migration 6: Create permission_audit_log table

```sql
CREATE TABLE permission_audit_log (
    id UUID NOT NULL DEFAULT gen_random_uuid(),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    event_type TEXT NOT NULL,
    actor_id UUID,
    resource_type TEXT,
    resource_id UUID,
    subject_type TEXT,
    subject_id UUID,
    permission_level TEXT,
    details JSONB NOT NULL DEFAULT '{}',
    PRIMARY KEY (timestamp, id)
) PARTITION BY RANGE (timestamp);

CREATE INDEX idx_audit_actor ON permission_audit_log(actor_id, timestamp DESC);
CREATE INDEX idx_audit_resource ON permission_audit_log(resource_type, resource_id, timestamp DESC);
```

---

## Materialized View (for Group Permissions)

```sql
CREATE MATERIALIZED VIEW user_group_permissions AS
SELECT
    gm.user_id,
    p.resource_type,
    p.resource_id,
    p.permission_level,
    p.id as permission_id,
    gm.group_id
FROM group_members gm
JOIN permissions p ON p.subject_type = 'group' AND p.subject_id = gm.group_id;

CREATE UNIQUE INDEX idx_ugp_lookup
    ON user_group_permissions(user_id, resource_type, resource_id, permission_level);

-- Refresh function
CREATE OR REPLACE FUNCTION refresh_user_group_permissions()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY user_group_permissions;
END;
$$ LANGUAGE plpgsql;
```
