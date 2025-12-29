# Research: User Profile & Settings Management

**Feature**: 004-user-profile-settings
**Date**: 2025-12-29

## Existing Infrastructure Analysis

### User Model (src/models/user.rs)

**Decision**: Reuse existing User model with all required fields already present.

**Rationale**: The User struct already contains:
- `display_name: String` (1-100 chars)
- `bio: Option<String>` (0-500 chars)
- `avatar_url: Option<String>`
- `measurement_pref: MeasurementPref` (Metric/Imperial enum)
- `email: Option<String>`, `email_verified: bool`
- `totp_enabled: bool`, `_totp_secret_encrypted: Option<Vec<u8>>`
- `deletion_requested_at: Option<DateTime<Utc>>`

**Alternatives considered**:
- Create separate UserSettings model → Rejected (unnecessary complexity, existing model sufficient)

### Account API (src/api/account.rs)

**Decision**: Extend existing account API with HTML handlers that call the same service layer.

**Rationale**: The API already provides:
- `GET /api/account/profile` - Get profile
- `POST /api/account/change-password` - Change password
- `POST /api/account/change-email` - Change email
- `GET /api/account/sessions` - List sessions
- `DELETE /api/account/sessions/{id}` - Revoke session
- `POST /api/account/2fa/*` - 2FA management
- `POST /api/account/delete` - Request deletion
- `POST /api/account/cancel-deletion` - Cancel deletion

**Alternatives considered**:
- Create separate settings API → Rejected (duplication, API already complete)

### Session Invalidation Pattern

**Decision**: Use existing session service to invalidate sessions on password change.

**Rationale**: Session table already exists with user_id foreign key. Adding `invalidate_other_sessions(user_id, current_session_id)` is straightforward.

**Alternatives considered**:
- Token versioning → More complex, not needed for this use case

## Content Deletion Strategy

**Decision**: Add `content_deletion_choice` field to deletion request flow.

**Rationale**:
- User choice captured at deletion request time
- Stored in deletion_requested_at + new content_choice column
- Background job processes based on choice after grace period

**Implementation approach**:
1. Add `deletion_content_choice` enum: `Anonymize | DeleteAll`
2. Modify deletion request to accept choice
3. Deletion processor handles each case:
   - Anonymize: Update author_id to "deleted user" sentinel UUID
   - DeleteAll: CASCADE delete or explicit cleanup

**Alternatives considered**:
- Always anonymize → Doesn't respect user's right to erasure
- Always delete → Loses community content value

## Template Architecture

**Decision**: Create dedicated settings template directory with tabbed navigation.

**Rationale**:
- Separation of concerns (profile vs security vs account)
- Reusable navigation component
- HTMX for in-place updates without full page reload

**Template structure**:
```
templates/settings/
├── layout.html      # Base layout with nav
├── profile.html     # Display name, bio, avatar, measurement pref
├── security.html    # Password, 2FA, sessions
└── account.html     # Email, deletion
```

**Alternatives considered**:
- Single monolithic settings page → Poor UX for mobile, hard to maintain
- SPA approach → Violates constitution (JS not required for critical paths)

## Accessibility Requirements

**Decision**: Form inputs with explicit labels, ARIA landmarks, visible focus states.

**Rationale**: Constitution IV requires WCAG 2.1 AA minimum.

**Implementation**:
- All inputs have `<label for="...">` associations
- Error messages linked via `aria-describedby`
- Settings sections use `<section>` with `aria-labelledby`
- Focus trap for deletion modal
- Skip link to main content

## Email Masking Pattern

**Decision**: Show first 2 characters + domain for email display.

**Rationale**: Balance between privacy and recognizability.

**Format**: `jo***@example.com` (for john@example.com)

**Alternatives considered**:
- Show full email → Privacy concern
- Hide completely → User can't verify which email is registered

## Dependencies

No new dependencies required. Feature uses:
- Axum 0.8 (existing)
- Askama (existing)
- SQLx (existing)
- validator (existing)
- TOTP via existing TotpService
