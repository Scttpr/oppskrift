# Handler Contracts: UI Parity

**Feature**: 006-ui-parity
**Date**: 2026-01-01

This document defines the HTML handler routes and their contracts. These are page endpoints, not API endpoints.

## Sessions Management

### GET /settings/security/sessions

**Description**: Display list of active sessions with revoke capability

**Authentication**: Required (AuthUser middleware)

**Template**: `templates/settings/sessions.html`

**Data provided to template**:
```rust
SessionsPageTemplate {
    user: ProfileView,
    sessions: Vec<SessionItemView>,
    current_session_id: Uuid,
    session_count: usize,
    csrf_token: String,
}
```

**Behavior**:
- List all active sessions for authenticated user
- Mark current session as non-revokable
- Show "Sign Out All Other Sessions" if count > 1

---

### POST /settings/security/sessions/{session_id}/revoke

**Description**: Revoke a specific session (HTMX endpoint)

**Authentication**: Required

**Request**: Session ID in path

**Response**:
- Success: Empty response with HX-Trigger for list refresh
- Error: Error message HTML fragment

**Validation**:
- Cannot revoke current session
- Session must belong to user

---

## Security Events

### GET /settings/security/events

**Description**: Display paginated security event log

**Authentication**: Required

**Template**: `templates/settings/security_events.html`

**Query Parameters**:
| Param | Type | Default | Description |
|-------|------|---------|-------------|
| page | u32 | 1 | Page number |

**Data provided to template**:
```rust
SecurityEventsPageTemplate {
    user: ProfileView,
    events: Vec<SecurityEventView>,
    page: u32,
    total_pages: u32,
    has_next: bool,
    has_prev: bool,
    csrf_token: String,
}
```

**Behavior**:
- Show 20 events per page (FR-007)
- Most recent first
- Show human-readable timestamps
- Icon per event type

---

## Privacy Settings

### GET /settings/privacy

**Description**: Display privacy settings (federation toggle, data export)

**Authentication**: Required

**Template**: `templates/settings/privacy.html`

**Data provided to template**:
```rust
PrivacyPageTemplate {
    user: ProfileView,
    federation_enabled: bool,
    recipe_count: i64,
    can_export_sync: bool,  // recipe_count <= 50
    csrf_token: String,
}
```

---

### POST /settings/privacy/federation

**Description**: Toggle federation on/off (HTMX endpoint)

**Authentication**: Required

**Request body**:
```rust
#[derive(Deserialize)]
struct FederationToggleForm {
    _csrf: String,
    enabled: bool,
}
```

**Response**: Updated toggle HTML fragment

**Behavior**:
- Update user.federation_enabled
- If disabling, warn about discoverability

---

### POST /settings/privacy/export

**Description**: Trigger data export

**Authentication**: Required

**Request body**:
```rust
#[derive(Deserialize)]
struct ExportForm {
    _csrf: String,
}
```

**Response**:
- If recipe_count <= 50: Direct JSON download
- If recipe_count > 50: Redirect to confirmation page, start async job

**Rate limit**: 1 export per hour

---

## Book Contributions

### GET /books/{id} (extended)

**Description**: Book view page, now includes contributions section

**Template**: `templates/books/view.html` (extended)

**Additional data**:
```rust
// Added to existing BookViewTemplate
contributions: Option<ContributionListView>,
is_owner: bool,
is_contributor: bool,
```

**Behavior**:
- Owner sees all contributions with accept/reject
- Contributor sees own contributions with status
- Others see accepted contributions only

---

### POST /books/{id}/contributions/{contribution_id}/accept

**Description**: Accept a contribution (HTMX endpoint, owner only)

**Authentication**: Required + Owner check

**Response**: Updated contribution row HTML

---

### POST /books/{id}/contributions/{contribution_id}/reject

**Description**: Reject a contribution (HTMX endpoint, owner only)

**Authentication**: Required + Owner check

**Request body**:
```rust
#[derive(Deserialize)]
struct RejectForm {
    _csrf: String,
    reason: Option<String>,  // Max 500 chars
}
```

**Response**: Updated contribution row HTML

**Side effect**: Notify contributor (FR-027)

---

## Followers/Following Lists

### GET /users/{id}/followers

**Description**: Display user's followers

**Authentication**: Optional (public pages)

**Template**: `templates/users/followers.html`

**Query Parameters**:
| Param | Type | Default | Description |
|-------|------|---------|-------------|
| page | u32 | 1 | Page number |

**Data provided to template**:
```rust
FollowersPageTemplate {
    profile_user: UserProfileView,
    users: Vec<UserCardView>,
    page: u32,
    total_pages: u32,
    has_next: bool,
    has_prev: bool,
    is_own_profile: bool,
    authenticated_user: Option<AuthUser>,
}
```

**Behavior**:
- Paginate at 100 users per page (SC-006)
- Show follow/unfollow button if authenticated
- Indicate mutual follows

---

### GET /users/{id}/following

**Description**: Display users that this user follows

**Template**: `templates/users/following.html`

**Same contract as /followers, different query**

---

## Route Registration Summary

```rust
// In src/handlers/settings.rs - add routes
.route("/security/events", get(security_events_page))
.route("/security/sessions/{session_id}/revoke", post(revoke_single_session))
.route("/privacy", get(privacy_page))
.route("/privacy/federation", post(toggle_federation))
.route("/privacy/export", post(export_data))

// In src/handlers/books.rs - add routes
.route("/{id}/contributions/{cid}/accept", post(accept_contribution))
.route("/{id}/contributions/{cid}/reject", post(reject_contribution))

// In src/handlers/users.rs - add routes
.route("/{id}/followers", get(followers_page))
.route("/{id}/following", get(following_page))
```
