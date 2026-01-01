# Research: UI Parity

**Feature**: 006-ui-parity
**Date**: 2026-01-01

## Summary

This feature primarily extends existing UI with templates and handlers. Research confirms existing infrastructure is sufficient with minimal gaps.

## Existing Infrastructure Analysis

### Sessions Management

**Decision**: Extend existing sessions.html template with individual session list

**Rationale**:
- `templates/settings/sessions.html` exists with session count display
- `src/api/account.rs` has `list_sessions` and `revoke_session` endpoints (lines 315-409)
- `SessionInfo` model already includes: id, device_info, ip_address, last_activity, is_current
- SessionService has `list_for_user()` and `revoke_by_id()` methods

**Gap**: Template shows count only, needs table with per-session revoke buttons

**Alternatives considered**:
- Create new standalone template → Rejected (sessions.html already exists)
- Use JavaScript SPA approach → Rejected (violates HTMX-first pattern)

---

### Security Events

**Decision**: Create new security_events.html template with pagination

**Rationale**:
- `GET /api/account/security-events` exists (src/api/account.rs:177-208)
- `SecurityEvent` struct defined with: id, event_type, ip_address, user_agent, metadata, created_at
- Query already supports limit parameter (default 50, max 100)
- security_events table exists in database

**Gap**: No HTML handler or template exists. Need:
- Handler in settings.rs for `/settings/security/events`
- Template at `templates/settings/security_events.html`
- Pagination using offset query parameter

**Pagination pattern**: Use existing `core/pagination.rs` module for consistency

---

### Federation Toggle

**Decision**: Add toggle to new privacy.html settings page

**Rationale**:
- `PATCH /api/v1/users/me/federation` exists (src/api/users.rs)
- User model has `federation_enabled` field
- WebFinger/ActivityPub endpoints already check federation status

**Gap**: No UI toggle exists. Need:
- Privacy settings page at `/settings/privacy`
- HTMX-powered toggle for federation
- Clear explanation text about federation implications

**Pattern**: Use same toggle pattern as 2FA enable/disable

---

### Data Export

**Decision**: Add export button to account.html settings page

**Rationale**:
- `GET /api/v1/users/me/export` exists (src/api/users.rs)
- Export returns JSON with all user data
- No async needed for <50 recipes (per clarification)

**Gap**: No UI trigger. Need:
- "Export My Data" button on account page
- For >50 recipes: async job with download link

**Async approach**: Use background job pattern from jobs/cleanup.rs

---

### Book Contributions

**Decision**: Extend books/view.html with contributions section

**Rationale**:
- `BookContribution` model exists (src/models/book_contribution.rs)
- `BookContributionService` exists (src/services/book_contribution_service.rs)
- API endpoints exist at `/api/v1/books/{id}/contributions`

**Gap**: No UI for viewing/managing contributions. Need:
- Contributions section in book view (owner sees all, contributor sees own)
- Accept/reject buttons with optional reason (HTMX)
- Status indicators (pending/accepted/rejected)

**Pattern**: Use same card pattern as recipe list

---

### Followers/Following Lists

**Decision**: Create new follower/following pages

**Rationale**:
- Follow model exists with follower_id, following_id
- FollowService has methods for getting followers/following
- FollowCounts already computed for profiles

**Gap**: No dedicated pages. Need:
- `/users/{id}/followers` page
- `/users/{id}/following` page
- User card component with follow/unfollow button

**Pattern**: Reuse user profile card pattern from activity_card.html

---

## Template Patterns

### Existing patterns to follow:

1. **Settings layout**: Use `{% extends "settings/_layout.html" %}` with `{% block settings_content %}`
2. **User pages**: Use `{% extends "layouts/base.html" %}` with standard blocks
3. **HTMX interactions**: Use `hx-post`, `hx-delete`, `hx-swap="outerHTML"` pattern
4. **Pagination**: Use query param `?page=N` with prev/next links
5. **CSRF protection**: Include `<input type="hidden" name="_csrf" value="{{ csrf_token }}" />`
6. **Accessibility**: Include proper ARIA labels, role attributes, focus states

### HTMX patterns from existing code:

```html
{# Delete with confirmation #}
<button
    hx-delete="/api/v1/resource/{id}"
    hx-confirm="Are you sure?"
    hx-target="closest .item"
    hx-swap="outerHTML"
>
    Delete
</button>

{# Toggle with swap #}
<button
    hx-patch="/api/v1/resource/{id}/toggle"
    hx-swap="outerHTML"
>
    Toggle
</button>
```

---

## Handler Patterns

### Existing handler structure:

```rust
#[derive(Template)]
#[template(path = "settings/page.html")]
struct PageTemplate {
    user: ProfileView,
    csrf_token: String,
    // page-specific fields
}

async fn page_handler(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    let csrf_token = generate_csrf_token(&state.db).await?;
    let user = UserService::get_by_id(&state.db, auth_user.id).await?;

    Ok(PageTemplate {
        user: ProfileView::from(user),
        csrf_token,
    })
}
```

---

## Security Considerations

1. **Session revocation**: Prevent current session self-revoke (already handled in API)
2. **Security events**: User can only see own events (WHERE user_id = $1)
3. **Data export**: Rate limit to prevent abuse (1 per hour suggested)
4. **Federation toggle**: Clear warning about discoverability implications
5. **Contribution rejection**: Sanitize rejection reason to prevent XSS

---

## Performance Considerations

1. **Pagination**: 20 items per page for security events (matches spec FR-007)
2. **Session list**: No pagination needed (users rarely have >10 sessions)
3. **Follower lists**: Paginate at 100 users (matches spec SC-006)
4. **Lazy loading**: Consider HTMX lazy loading for contribution lists on large books

---

## No Unknowns Remaining

All NEEDS CLARIFICATION items from spec were resolved in clarification session:
- Security events retention: 90 days ✓
- Async export threshold: 50 recipes ✓
- Contribution rejection workflow: Notify with reason, allow re-submit ✓
