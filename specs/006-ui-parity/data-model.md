# Data Model: UI Parity

**Feature**: 006-ui-parity
**Date**: 2026-01-01

## Overview

This feature does not introduce new database entities. It exposes existing entities through UI. This document clarifies the entities being surfaced and any view models needed for templates.

## Existing Entities (No Changes)

### Session

**Table**: `sessions` (existing)

| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Primary key |
| user_id | UUID | FK to users |
| token_hash | VARCHAR | Hashed session token |
| device_info | VARCHAR | User-agent derived info |
| ip_address | INET | Client IP |
| created_at | TIMESTAMP | Session creation |
| last_activity | TIMESTAMP | Last request time |
| expires_at | TIMESTAMP | Expiration time |

**Rust model**: `SessionInfo` in `src/models/session.rs`

---

### SecurityEvent

**Table**: `security_events` (existing)

| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Primary key |
| user_id | UUID | FK to users |
| event_type | VARCHAR | Event type (login, password_change, etc.) |
| ip_address | INET | Client IP |
| user_agent | VARCHAR | Browser info |
| metadata | JSONB | Additional context |
| created_at | TIMESTAMP | Event timestamp |

**Rust model**: `SecurityEvent` in `src/api/account.rs`

**Retention**: 90 days (from clarification)

---

### Follow

**Table**: `follows` (existing)

| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Primary key |
| follower_id | UUID | FK to users (who follows) |
| following_id | UUID | FK to users (who is followed) |
| created_at | TIMESTAMP | When follow occurred |
| ap_id | VARCHAR | ActivityPub ID |

**Rust model**: `Follow` in `src/models/follow.rs`

---

### BookContribution

**Table**: `book_contributions` (existing)

| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Primary key |
| book_id | UUID | FK to recipe_books |
| recipe_id | UUID | FK to recipes |
| contributor_id | UUID | FK to users |
| added_at | TIMESTAMP | Contribution timestamp |

**Rust model**: `BookContribution` in `src/models/book_contribution.rs`

**Note**: Current model lacks status field for pending/accepted/rejected. See Extensions below.

---

### User (federation field)

**Table**: `users` (existing)

| Field | Type | Description |
|-------|------|-------------|
| federation_enabled | BOOLEAN | Whether user is federated |

Used by federation toggle.

---

## Required Schema Extensions

### BookContribution Status

The current `book_contributions` table lacks a status field. Need migration to add:

```sql
-- Migration: add_contribution_status.sql
ALTER TABLE book_contributions
ADD COLUMN status VARCHAR(20) NOT NULL DEFAULT 'accepted';

ALTER TABLE book_contributions
ADD COLUMN rejection_reason TEXT;

-- Add constraint for valid statuses
ALTER TABLE book_contributions
ADD CONSTRAINT chk_contribution_status
CHECK (status IN ('pending', 'accepted', 'rejected'));
```

**Rationale**: FR-017 requires viewing contributions with status; FR-026/FR-027 require rejection with reason.

---

## View Models (Template DTOs)

### SessionListView

Used by sessions.html template:

```rust
pub struct SessionListView {
    pub sessions: Vec<SessionItemView>,
    pub current_session_id: Uuid,
    pub total_count: usize,
    pub csrf_token: String,
}

pub struct SessionItemView {
    pub id: Uuid,
    pub device_info: String,      // Parsed user-agent
    pub ip_address: String,       // Formatted IP
    pub last_activity: String,    // Human-readable "2 hours ago"
    pub created_at: String,       // ISO date
    pub is_current: bool,
}
```

---

### SecurityEventsView

Used by security_events.html template:

```rust
pub struct SecurityEventsView {
    pub events: Vec<SecurityEventView>,
    pub page: u32,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
    pub csrf_token: String,
}

pub struct SecurityEventView {
    pub id: Uuid,
    pub event_type: String,       // Human-readable label
    pub event_icon: String,       // SVG path for event type
    pub ip_address: String,
    pub device_info: String,      // From user_agent
    pub timestamp: String,        // Human-readable
    pub metadata_summary: String, // One-line summary of metadata
}
```

---

### ContributionListView

Used by books/view.html contributions section:

```rust
pub struct ContributionListView {
    pub contributions: Vec<ContributionItemView>,
    pub is_owner: bool,
    pub can_contribute: bool,
    pub csrf_token: String,
}

pub struct ContributionItemView {
    pub id: Uuid,
    pub recipe_id: Uuid,
    pub recipe_title: String,
    pub contributor_id: Uuid,
    pub contributor_name: String,
    pub contributor_avatar: Option<String>,
    pub status: String,           // pending, accepted, rejected
    pub rejection_reason: Option<String>,
    pub added_at: String,
}
```

---

### FollowerListView

Used by followers.html and following.html:

```rust
pub struct FollowerListView {
    pub users: Vec<UserCardView>,
    pub total_count: i64,
    pub page: u32,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
    pub is_own_profile: bool,
    pub list_type: String,        // "followers" or "following"
}

pub struct UserCardView {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub bio_excerpt: Option<String>,  // First 100 chars
    pub is_following: bool,           // Current user follows this person
    pub follows_you: bool,            // This person follows current user
}
```

---

### PrivacySettingsView

Used by privacy.html:

```rust
pub struct PrivacySettingsView {
    pub federation_enabled: bool,
    pub recipe_count: i64,        // For export threshold check
    pub export_available: bool,   // Not rate-limited
    pub csrf_token: String,
}
```

---

## Entity Relationships

```
User (1) ─────< (N) Session
User (1) ─────< (N) SecurityEvent
User (1) ─────< (N) Follow (as follower)
User (1) ─────< (N) Follow (as following)
User (1) ─────< (N) BookContribution (as contributor)
RecipeBook (1) ─────< (N) BookContribution
Recipe (1) ─────< (N) BookContribution
```

---

## Validation Rules

| Entity | Field | Rule |
|--------|-------|------|
| BookContribution | status | Must be: pending, accepted, rejected |
| BookContribution | rejection_reason | Max 500 chars; required if status=rejected |
| SecurityEvent | event_type | Must be known type |

---

## State Transitions

### BookContribution Status

```
[pending] ──accept──> [accepted]
[pending] ──reject──> [rejected]
[rejected] ──resubmit──> [pending]
```

- Only owner can accept/reject
- Only contributor can resubmit
- Accepted contributions cannot be changed (remove instead)
