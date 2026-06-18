# Contributing to Oppskrift

## Development Setup

### Prerequisites

- Rust 1.85+ (stable) — some dependencies require edition 2024
- Docker or Podman with Compose
- Tailwind CSS CLI (for CSS changes)

### Quick Start

```bash
# Clone the repository
git clone https://github.com/scttpr/oppskrift.git
cd oppskrift

# Copy environment file
cp .env.example .env

# Install git hooks
./.githooks/install.sh

# First-time setup (starts database and runs migrations)
make setup

# Run the app with auto-reload
make dev
```

The app will be available at http://localhost:3000

## Git Hooks

Pre-commit hooks ensure code quality before pushing:

```bash
# Install hooks (run once after cloning)
./.githooks/install.sh
```

The pre-commit hook runs:
- `gitleaks` - Secret scanning on staged files
- `make lint` - Clippy and format check (requires database)

## Makefile Commands

### Development

```bash
make setup       # First-time setup (db + migrations)
make dev         # Run with auto-reload (requires cargo-watch)
make css         # Build Tailwind CSS
make css-watch   # Watch CSS for changes
```

### Database

```bash
make db          # Start database container
make migrate     # Run migrations
make reset-db    # Drop, recreate, and migrate
```

### Quality

```bash
make lint        # Run clippy + format check
make test        # Run all tests
make fmt         # Format code
```

### Cleanup

```bash
make clean       # Clean build artifacts and stop containers
```

## SQLx and Database

SQLx verifies queries at compile time. A query cache is committed under `.sqlx/`,
so builds work offline (`SQLX_OFFLINE=true`, as used by the Docker and CI builds)
without a database. When you add or change a `sqlx::query!` macro, regenerate the
cache against a running database and commit the result:

```bash
# Start database and run migrations
make setup

# Regenerate and commit the SQLx query cache after changing queries
cargo sqlx prepare
```

## Code Style

### Rust

- Follow Rust standard formatting (rustfmt)
- Use clippy with `-D warnings` (treat warnings as errors)
- Prefer descriptive variable names
- Write doc comments for public items
- Unit tests inline with `#[cfg(test)] mod tests { ... }`

### Commits

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): description

Types:
- feat: New feature
- fix: Bug fix
- docs: Documentation
- chore: Maintenance
- refactor: Code refactoring
- test: Test changes
- perf: Performance improvement
- security: Security hardening
```

### Pull Requests

1. Create a feature branch from `main`
2. Make focused, atomic commits
3. Write/update tests as needed
4. Ensure `make lint` and `make test` pass
5. Request review

## Project Structure

```
src/
├── api/          # REST API endpoints (auth, account, users, recipes,
│                 #   books, groups, social, activitypub, feeds, webfinger,
│                 #   oembed, openapi) + middleware/
├── handlers/     # HTML page handlers
├── services/     # Business logic (auth, password, session, email,
│                 #   recipe, book, group, permission, totp, image, export, ...)
├── models/       # Database models
└── core/         # Shared utilities (config, db, error, crypto, csrf,
                  #   pagination, audit, activitypub/, seeds/)

templates/        # Askama HTML templates
static/           # CSS, JS (HTMX vendored)
migrations/       # SQLx database migrations
tests/            # Integration and security tests
```

## Testing

```bash
# Run all tests
make test

# Run specific test
cargo test test_name

# Run with logging
RUST_LOG=debug cargo test

# Run specific module tests
cargo test models::recipe

# Run auth tests only
cargo test --test auth_test --test security_auth_test
```

## Adding New Features

1. Create migration if needed: `sqlx migrate add feature_name`
2. Run migrations: `make migrate`
3. Add model in `src/models/`
4. Add service in `src/services/`
5. Add API endpoint in `src/api/`
6. Add tests inline with `#[cfg(test)]`

## License

This project is licensed under AGPL-3.0-or-later. Contributions are welcome under the same license.
