# Contributing to Oppskrift

## Development Setup

### Prerequisites

- Rust 1.75+ (stable)
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

# Start database
make db

# Run migrations
make migrate

# Run the app
make run
```

The app will be available at http://localhost:3000

### Alternative: Full Docker Setup

```bash
# Start everything (db, minio, app)
make up
```

## Git Hooks

Pre-commit hooks ensure code quality before pushing:

```bash
# Install hooks (run once after cloning)
./.githooks/install.sh
```

The pre-commit hook runs:
- `cargo clippy --all-features -- -D warnings`
- `cargo fmt -- --check`

## Makefile Commands

### Development

```bash
make run         # Build CSS and run app
make dev         # Run with auto-reload (requires cargo-watch)
make css         # Build Tailwind CSS
make css-watch   # Watch CSS for changes
```

### Database

```bash
make db          # Start database container
make db-stop     # Stop database container
make migrate     # Run migrations
make seed        # Seed with test data
make reset-db    # Drop, recreate, migrate, and seed
```

### Docker/Podman

```bash
make up          # Build and start all services
make rebuild     # Full rebuild (no cache)
make down        # Stop all services
```

### Quality

```bash
make lint        # Run clippy + format check
make test        # Run all tests
make check       # Compile check only
make fmt         # Format code
make audit       # Security audit
```

### Build

```bash
make build       # Build release binary
make clean       # Clean build artifacts
```

## SQLx Offline Mode

For faster local checks without a database connection:

```bash
# Generate offline cache (with database running)
cargo sqlx prepare

# Commit the cache
git add .sqlx
```

The Docker build uses offline mode automatically.

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
├── api/          # REST API endpoints
│   ├── auth.rs       # Authentication (register, login, logout)
│   ├── account.rs    # Account management
│   ├── recipes.rs    # Recipe CRUD
│   ├── books.rs      # Recipe books
│   ├── social.rs     # Follow, save, share
│   ├── activitypub.rs# Federation
│   ├── feeds.rs      # RSS/Atom
│   ├── webfinger.rs  # Discovery
│   └── oembed.rs     # Embeds
├── handlers/     # HTML page handlers
├── services/     # Business logic
│   ├── auth_service.rs    # Registration, login, sessions
│   ├── password_service.rs# Password hashing
│   └── session_service.rs # Session management
├── models/       # Database models
├── jobs/         # Background jobs
└── lib/          # Shared utilities

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
cargo test --test login_test --test registration_test
```

## Adding New Features

1. Create migration if needed: `sqlx migrate add feature_name`
2. Add model in `src/models/`
3. Add service in `src/services/`
4. Add API endpoint in `src/api/`
5. Add tests inline with `#[cfg(test)]`
6. Update SQLx offline cache: `cargo sqlx prepare`

## License

This project is licensed under AGPL-3.0-or-later. Contributions are welcome under the same license.
