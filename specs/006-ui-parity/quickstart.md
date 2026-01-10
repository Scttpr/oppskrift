# Quickstart: UI Parity

**Feature**: 006-ui-parity
**Date**: 2026-01-01

## Prerequisites

- Rust 1.75+ installed
- PostgreSQL 15+ running with Oppskrift database
- Environment variables configured (see `.env.example`)

## Quick Setup

```bash
# 1. Ensure on feature branch
git checkout 006-ui-parity

# 2. Apply any new migrations
make migrate

# 3. Run development server
cargo run

# 4. Access in browser
open http://localhost:3000
```

## Development Workflow

### Running Tests

```bash
# All tests
cargo test

# Specific test file
cargo test ui_parity

# With output
cargo test -- --nocapture
```

### Template Development

Templates use Askama with compile-time checking:

```bash
# Check templates compile
cargo check

# Hot reload not available - restart server after template changes
cargo run
```

### HTMX Development

1. Edit template in `templates/`
2. Add HTMX attributes (`hx-*`)
3. Restart server
4. Test in browser with network tab open

## Key Files to Edit

### Phase 1: Sessions List Enhancement

```
templates/settings/sessions.html    # Extend with session table
src/handlers/settings.rs            # Add revoke_single_session handler
```

### Phase 2: Security Events Page

```
templates/settings/security_events.html  # NEW
src/handlers/settings.rs                 # Add security_events_page handler
```

### Phase 3: Privacy Settings

```
templates/settings/privacy.html     # NEW
templates/settings/_nav.html        # Add Privacy link
src/handlers/settings.rs            # Add privacy handlers
```

### Phase 4: Book Contributions

```
templates/books/view.html           # Extend with contributions section
src/handlers/books.rs               # Add contribution handlers
migrations/YYYYMMDD_*.sql           # Add status column
```

### Phase 5: Followers/Following

```
templates/users/followers.html      # NEW
templates/users/following.html      # NEW
src/handlers/users.rs               # Add followers/following handlers
```

## Testing Each Feature

### Sessions Management

1. Login from multiple browsers/incognito
2. Go to Settings > Security > Sessions
3. Verify all sessions listed
4. Click Revoke on non-current session
5. Verify session removed, other browser logged out

### Security Events

1. Perform login, change password, enable 2FA
2. Go to Settings > Security > Security Events
3. Verify events appear with correct details
4. Test pagination with ?page=2

### Federation Toggle

1. Go to Settings > Privacy
2. Toggle federation off
3. Verify WebFinger returns 404 for user
4. Toggle on, verify discoverable again

### Data Export

1. Go to Settings > Privacy
2. Click "Export My Data"
3. If <50 recipes: verify immediate JSON download
4. If >50 recipes: verify async notification

### Book Contributions

1. Create a book as User A
2. As User B, contribute a recipe (via API for now)
3. As User A, view book, see contribution
4. Accept or reject with reason
5. As User B, verify notification received

### Followers/Following

1. Follow several users
2. Go to your profile > Followers
3. Verify list shows followers
4. Go to Following tab
5. Verify unfollow button works

## Common Issues

### Template Compilation Errors

```
error: cannot find template
```

Fix: Ensure file is in `templates/` with `.html` extension and matches `#[template(path = "...")]`

### CSRF Token Issues

```
403 Forbidden: Invalid CSRF token
```

Fix: Ensure form includes `<input type="hidden" name="_csrf" value="{{ csrf_token }}" />`

### HTMX Not Working

Check:
1. `hx-*` attributes are lowercase
2. Target element exists
3. Server returns HTML fragment, not full page
4. Check browser console for HTMX errors

## Environment Variables

Required for this feature:

```bash
DATABASE_URL=postgres://oppskrift:oppskrift@localhost:5432/oppskrift
BASE_URL=http://localhost:3000
JWT_SECRET=change-me-in-production-min-32-chars
TOTP_ENCRYPTION_KEY=0123456789abcdef...  # 64 hex chars
```

## Useful Commands

```bash
# Check database migrations status
sqlx migrate info

# Run specific migration
sqlx migrate run

# Revert last migration
sqlx migrate revert

# Check for clippy warnings
cargo clippy

# Format code
cargo fmt
```
