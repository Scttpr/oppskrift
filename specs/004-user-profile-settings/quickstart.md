# Quickstart: User Profile & Settings Management

**Feature**: 004-user-profile-settings
**Date**: 2025-12-29

## Overview

This feature adds user-facing settings pages for profile management, security settings, and account lifecycle. It leverages existing API infrastructure to provide HTML interfaces.

## Prerequisites

- Rust 1.75+ installed
- PostgreSQL 15+ running
- Existing database with user tables migrated
- Development environment configured (see main CONTRIBUTING.md)

## Quick Start

```bash
# Checkout feature branch
git checkout 004-user-profile-settings

# Run migrations (adds deletion_content_choice)
sqlx migrate run

# Start development server
cargo run

# Access settings at http://localhost:3000/settings
```

## Key Files

### New Files

| Path | Purpose |
|------|---------|
| `src/handlers/settings.rs` | Settings page handlers |
| `templates/settings/profile.html` | Profile view/edit template |
| `templates/settings/security.html` | Password, 2FA, sessions template |
| `templates/settings/account.html` | Email, deletion template |
| `templates/settings/_nav.html` | Settings navigation partial |
| `tests/settings_test.rs` | Handler integration tests |

### Modified Files

| Path | Change |
|------|--------|
| `src/handlers/mod.rs` | Add settings routes |
| `src/models/user.rs` | Add DeletionContentChoice enum |
| `src/api/account.rs` | Extend delete request with content_choice |
| `migrations/YYYYMMDD_add_deletion_content_choice.sql` | New migration |

## Routes

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| GET | `/settings` | `settings_page` | Redirect to profile |
| GET | `/settings/profile` | `profile_page` | View/edit profile |
| POST | `/settings/profile` | `update_profile` | Save profile changes |
| GET | `/settings/security` | `security_page` | Password, 2FA, sessions |
| GET | `/settings/account` | `account_page` | Email, deletion |

## Example Code

### Handler Pattern

```rust
// src/handlers/settings.rs
use askama::Template;
use axum::{extract::State, response::Html, routing::get, Router};
use crate::api::middleware::AuthUser;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(|| async { Redirect::to("/settings/profile") }))
        .route("/profile", get(profile_page).post(update_profile))
        .route("/security", get(security_page))
        .route("/account", get(account_page))
}

#[derive(Template)]
#[template(path = "settings/profile.html")]
struct ProfileTemplate {
    user: UserProfile,
    measurement_options: Vec<(&'static str, &'static str)>,
}

async fn profile_page(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Html<String>> {
    let user = UserService::get_by_id(&state.db, auth.id).await?;
    let template = ProfileTemplate {
        user: UserProfile::from(user),
        measurement_options: vec![("metric", "Metric"), ("imperial", "Imperial")],
    };
    Ok(Html(template.render()?))
}
```

### Template Pattern

```html
<!-- templates/settings/profile.html -->
{% extends "settings/_layout.html" %}

{% block content %}
<form method="post" action="/settings/profile" class="space-y-4">
    <div>
        <label for="display_name" class="block text-sm font-medium">
            Display Name
        </label>
        <input
            type="text"
            id="display_name"
            name="display_name"
            value="{{ user.display_name }}"
            maxlength="100"
            required
            class="mt-1 block w-full rounded-md border-gray-300"
            aria-describedby="display_name_help"
        >
        <p id="display_name_help" class="mt-1 text-sm text-gray-500">
            1-100 characters
        </p>
    </div>

    <!-- Bio field -->
    <div>
        <label for="bio" class="block text-sm font-medium">Bio</label>
        <textarea
            id="bio"
            name="bio"
            maxlength="500"
            rows="3"
            class="mt-1 block w-full rounded-md border-gray-300"
        >{{ user.bio|default("") }}</textarea>
    </div>

    <button type="submit" class="btn btn-primary">
        Save Changes
    </button>
</form>
{% endblock %}
```

### Test Pattern

```rust
// tests/settings_test.rs
use common::TestContext;

#[tokio::test]
async fn test_profile_page_requires_auth() {
    let ctx = TestContext::new().await;
    let response = ctx.server.get("/settings/profile").await;
    assert!(response.status_code() == 302 || response.status_code() == 401);
}

#[tokio::test]
async fn test_profile_page_shows_user_data() {
    let mut ctx = TestContext::new().await;
    let (user_id, session) = ctx.create_and_login("testuser").await;

    let response = ctx.server
        .get("/settings/profile")
        .add_cookie(cookie::Cookie::new("oppskrift_session", session))
        .await;

    assert_eq!(response.status_code(), 200);
    let body = response.text();
    assert!(body.contains("testuser"));

    ctx.cleanup().await;
}
```

## Testing

```bash
# Run all tests
cargo test

# Run only settings tests
cargo test settings

# Run with coverage
cargo tarpaulin --out Html
```

## Accessibility Checklist

- [ ] All form inputs have associated labels
- [ ] Error messages use `aria-describedby`
- [ ] Focus visible on all interactive elements
- [ ] Settings sections use `<section>` with `aria-labelledby`
- [ ] Deletion modal has focus trap
- [ ] Skip link to main content
- [ ] Color not sole means of conveying information
