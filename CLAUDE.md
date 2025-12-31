# Oppskrift Development Guidelines

Auto-generated from all feature plans. Last updated: 2025-12-25

## Active Technologies
- Rust 1.75+ + Axum 0.8, SQLx 0.8, argon2, totp-rs, lettre, tower_governor (002-user-auth)
- PostgreSQL 15+ (sessions, tokens, security_events tables) (002-user-auth)
- Rust 1.75+ + Axum 0.8, SQLx 0.8, tokio, activitypub_federation 0.6 (003-test-coverage)
- PostgreSQL 15+ (via SQLx) (003-test-coverage)
- Rust 1.75+ + Axum 0.8, Askama (templates), SQLx (database), validator (input validation) (004-user-profile-settings)
- PostgreSQL 15+ (existing schema) (004-user-profile-settings)
- Rust 2021 edition (1.75+) + Axum 0.8, SQLx 0.8, tokio 1.x, tower-http 0.6, validator 0.20, activitypub_federation 0.6 (005-abac-authorization)
- PostgreSQL 15+ (via SQLx with offline mode) (005-abac-authorization)

- **Language**: Rust 1.75+
- **Framework**: Axum 0.8
- **Database**: PostgreSQL 15+ with SQLx
- **Templates**: Askama (compile-time checked)
- **Frontend**: Server-rendered HTML + HTMX
- **Federation**: activitypub-federation-rust 0.6

## Project Structure

```text
src/
├── models/              # Data models (unit tests inline with #[cfg(test)])
├── services/            # Business logic
├── api/                 # REST/JSON API endpoints
├── handlers/            # HTML page handlers
└── lib/                 # Shared utilities

templates/               # Askama templates (.html)
├── layouts/
├── recipes/
├── books/
└── components/

static/                  # CSS, JS (HTMX vendored)
tests/                   # Integration tests only
```

## Commands

```bash
cargo build              # Build
cargo test               # Run all tests
cargo clippy             # Lint
cargo fmt                # Format
cargo run                # Run dev server
```

## Code Style

- Follow standard Rust conventions (rustfmt defaults)
- Unit tests inline with `#[cfg(test)] mod tests { ... }`
- Integration tests in `tests/` directory
- Use `thiserror` for library errors, `anyhow` for application errors

## Recent Changes
- 005-abac-authorization: Added Rust 2021 edition (1.75+) + Axum 0.8, SQLx 0.8, tokio 1.x, tower-http 0.6, validator 0.20, activitypub_federation 0.6
- 004-user-profile-settings: Added Rust 1.75+ + Axum 0.8, Askama (templates), SQLx (database), validator (input validation)
- 003-test-coverage: Added Rust 1.75+ + Axum 0.8, SQLx 0.8, tokio, activitypub_federation 0.6


<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
