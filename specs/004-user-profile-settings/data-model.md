# Data Model: User Profile & Settings Management

**Feature**: 004-user-profile-settings
**Date**: 2025-12-29

## Entities

### User (existing - src/models/user.rs)

Primary entity for profile and account data. Already exists with all required fields.

| Field | Type | Constraints | Notes |
|-------|------|-------------|-------|
| id | UUID | PK | Immutable |
| username | String | Unique, 3-30 chars | Immutable after creation |
| email | Option<String> | Unique when present | Nullable for federated users |
| email_verified | bool | Default false | |
| password_hash | Option<String> | | Nullable for federated users |
| display_name | String | 1-100 chars | Required |
| bio | Option<String> | 0-500 chars | |
| avatar_url | Option<String> | Valid URL format | |
| measurement_pref | MeasurementPref | Enum: Metric/Imperial | Default: Metric |
| totp_enabled | bool | Default false | |
| totp_secret_encrypted | Option<Vec<u8>> | | Encrypted at rest |
| deletion_requested_at | Option<DateTime> | | Null = not scheduled |
| created_at | DateTime | | Immutable |
| updated_at | DateTime | | Auto-updated |
| ap_id | String | Unique | ActivityPub identifier |
| federation_enabled | bool | Default true | |

### Session (existing - sessions table)

Tracks active login sessions for session management.

| Field | Type | Constraints | Notes |
|-------|------|-------------|-------|
| id | UUID | PK | Session identifier |
| user_id | UUID | FK → users | |
| token_hash | String | | Hashed session token |
| user_agent | Option<String> | | Browser/device info |
| ip_address | Option<String> | | Last known IP |
| created_at | DateTime | | Login time |
| last_activity | DateTime | | Last request time |
| expires_at | DateTime | | Session expiry |

### DeletionContentChoice (new enum)

User's choice for content handling during account deletion.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "deletion_content_choice", rename_all = "snake_case")]
pub enum DeletionContentChoice {
    Anonymize,  // Keep content, replace author with "Deleted User"
    DeleteAll,  // Delete all user's content
}
```

### Migration: Add deletion_content_choice

```sql
-- Add deletion content choice to users table
CREATE TYPE deletion_content_choice AS ENUM ('anonymize', 'delete_all');

ALTER TABLE users
ADD COLUMN deletion_content_choice deletion_content_choice;
```

## Relationships

```
User 1 ──────< Session (one user has many sessions)
User 1 ──────< Recipe (author_id - existing)
User 1 ──────< RecipeBook (owner_id - existing)
```

## State Transitions

### Account Deletion Flow

```
Normal → Deletion Requested → (grace period) → Deleted
  ↑              │
  └──────────────┘
     (cancel)
```

| State | deletion_requested_at | deletion_content_choice |
|-------|----------------------|------------------------|
| Normal | NULL | NULL |
| Deletion Requested | timestamp | 'anonymize' or 'delete_all' |
| Deleted | N/A (row deleted) | N/A |

### 2FA Flow

```
Disabled → Setup Initiated → Enabled
              │                  │
              └──────────────────┘
                   (cancel/disable)
```

## Validation Rules

### Profile Updates (UpdateUser)

| Field | Rule | Error Message |
|-------|------|---------------|
| display_name | 1-100 chars | "Display name must be 1-100 characters" |
| bio | 0-500 chars | "Bio must be at most 500 characters" |
| avatar_url | Valid URL or empty | "Avatar URL must be a valid URL" |
| measurement_pref | Metric or Imperial | "Invalid measurement preference" |

### Password Change

| Field | Rule | Error Message |
|-------|------|---------------|
| current_password | Must match stored hash | "Current password is incorrect" |
| new_password | Min 8 chars, uppercase, lowercase, digit, special | "Password does not meet requirements" |
| confirm_password | Must match new_password | "Passwords do not match" |

### Email Change

| Field | Rule | Error Message |
|-------|------|---------------|
| new_email | Valid email format | "Invalid email format" |
| new_email | Not already registered | "Email is already in use" |
| current_password | Must match stored hash | "Current password is incorrect" |

## Indexes

Existing indexes sufficient:
- `users_email_key` - Unique on email
- `users_username_key` - Unique on username
- `sessions_user_id_idx` - For session lookups by user
- `sessions_token_hash_idx` - For session validation

## Data Volume Estimates

- Users: Existing, no change
- Sessions per user: ~3-5 active (devices)
- Settings changes: Low frequency (<1/day/active user)
