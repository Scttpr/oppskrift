# Contributing to Oppskrift

## Development Setup

### Prerequisites

- Rust 1.75+ (stable)
- PostgreSQL 15+
- Docker and Docker Compose (optional)
- Tailwind CSS CLI

### Quick Start

1. Clone the repository:
   ```bash
   git clone https://github.com/scttpr/oppskrift.git
   cd oppskrift
   ```

2. Copy environment file:
   ```bash
   cp .env.example .env
   ```

3. Start PostgreSQL (Docker):
   ```bash
   docker-compose up -d postgres
   ```

4. Run migrations:
   ```bash
   sqlx migrate run
   ```

5. Build CSS:
   ```bash
   make css
   ```

6. Run the server:
   ```bash
   cargo run
   ```

### Makefile Commands

```bash
make dev       # Run development server with auto-reload
make css       # Build Tailwind CSS
make seed      # Seed database with test data
make reset-db  # Drop, migrate, and seed database
make test      # Run tests
make lint      # Run clippy and rustfmt
```

## Code Style

### Rust

- Follow Rust standard formatting (rustfmt)
- Use clippy with default lints
- Prefer descriptive variable names
- Write doc comments for public items

### Commits

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): description

- feat: New feature
- fix: Bug fix
- docs: Documentation
- chore: Maintenance
- refactor: Code refactoring
- test: Test changes
- perf: Performance improvement
```

### Pull Requests

1. Create a feature branch from `main`
2. Make focused, atomic commits
3. Write/update tests as needed
4. Ensure CI passes
5. Request review

## Project Structure

```
src/
├── api/          # REST API endpoints
├── handlers/     # HTML page handlers
├── jobs/         # Background job processing
├── lib/          # Shared utilities
├── models/       # Database models
└── services/     # Business logic

templates/        # Askama HTML templates
static/           # Static assets (CSS, JS)
migrations/       # SQLx database migrations
```

## Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with logging
RUST_LOG=debug cargo test
```

## License

This project is licensed under AGPL-3.0. Contributions are welcome under the same license.
