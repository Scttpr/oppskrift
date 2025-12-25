# Oppskrift Development Guidelines

Auto-generated from all feature plans. Last updated: 2025-12-25

## Active Technologies

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

- 001-recipe-sharing: Initial feature - recipes, books, sharing, ActivityPub federation

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
