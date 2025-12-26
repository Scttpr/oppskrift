# Oppskrift

A federated recipe sharing platform built with Rust and ActivityPub.

## Features

- **Recipe Management**: Create, edit, and organize recipes with ingredients, instructions, and images
- **Recipe Books**: Organize recipes into collections
- **Social Features**: Follow users, save recipes, share with activity feed
- **Federation**: ActivityPub support for cross-instance recipe sharing
- **RSS/Atom Feeds**: Subscribe to user recipes via feed readers
- **WebFinger**: Discover users across federated instances
- **oEmbed**: Rich embeds for recipes shared on other platforms

## Tech Stack

- **Backend**: Rust with Axum 0.8
- **Database**: PostgreSQL 15+ with SQLx
- **Templates**: Askama (compile-time checked)
- **Frontend**: Server-rendered HTML + HTMX + Tailwind CSS
- **Storage**: S3-compatible (MinIO for development)
- **Federation**: activitypub-federation-rust 0.6

## Quick Start

### Prerequisites

- Rust 1.75+ (stable)
- Docker/Podman with Compose
- Tailwind CSS CLI (optional, for CSS changes)

### Development Setup

```bash
# Clone and enter directory
git clone https://github.com/scttpr/oppskrift.git
cd oppskrift

# Copy environment file
cp .env.example .env

# Start all services (db, minio, app)
make up

# Or for local development without Docker app:
make db          # Start database only
make migrate     # Run migrations
make run         # Run app locally
```

The app will be available at http://localhost:3000

### Makefile Commands

```bash
# Development
make run         # Build CSS and run app
make dev         # Run with auto-reload (requires cargo-watch)
make css         # Build Tailwind CSS
make css-watch   # Watch CSS for changes

# Database
make db          # Start database container
make db-stop     # Stop database container
make migrate     # Run migrations
make seed        # Seed with test data
make reset-db    # Drop, recreate, migrate, and seed

# Docker/Podman
make up          # Build and start all services
make rebuild     # Full rebuild (no cache)
make down        # Stop all services

# Quality
make lint        # Run clippy + format check
make test        # Run all tests
make check       # Compile check only
make fmt         # Format code
make audit       # Security audit

# Build
make build       # Build release binary
make clean       # Clean build artifacts
```

## Project Structure

```
src/
├── api/          # REST/JSON API endpoints
│   ├── auth.rs       # Authentication (JWT)
│   ├── recipes.rs    # Recipe CRUD
│   ├── books.rs      # Recipe book management
│   ├── social.rs     # Follow, save, share
│   ├── activitypub.rs # Federation endpoints
│   ├── feeds.rs      # RSS/Atom feeds
│   ├── webfinger.rs  # WebFinger discovery
│   └── oembed.rs     # oEmbed provider
├── handlers/     # HTML page handlers
├── services/     # Business logic layer
├── models/       # Database models
├── jobs/         # Background job processing
└── lib/          # Shared utilities
    ├── activitypub/  # AP protocol implementation
    ├── audit.rs      # Security audit logging
    ├── config.rs     # Configuration
    ├── error.rs      # Error types
    └── pagination.rs # Pagination helpers

templates/        # Askama HTML templates
static/           # CSS, JS (HTMX vendored)
migrations/       # SQLx database migrations
```

## API Endpoints

### REST API (`/api/v1`)

- `POST /auth/login` - Authenticate and get JWT token
- `GET/POST/PUT/DELETE /recipes` - Recipe CRUD
- `GET/POST/PUT/DELETE /books` - Recipe book CRUD
- `POST /users/{id}/follow` - Follow a user
- `POST /recipes/{id}/save` - Save a recipe
- `GET /feed` - Activity feed

### Federation (`/ap`)

- `GET /users/{id}` - Actor profile (Person)
- `POST /users/{id}/inbox` - Receive activities
- `GET /users/{id}/outbox` - User's activities
- `GET /recipes/{id}` - Recipe object
- `GET /books/{id}` - Book collection

### Discovery

- `GET /.well-known/webfinger` - WebFinger
- `GET /oembed` - oEmbed provider
- `GET /feeds/recipes.rss` - Public recipes RSS
- `GET /feeds/users/{id}/recipes.atom` - User recipes Atom

## Environment Variables

See `.env.example` for all options. Required:

```bash
DATABASE_URL=postgres://user:pass@localhost:5432/oppskrift
JWT_SECRET=your-secret-min-32-chars
S3_BUCKET=oppskrift
```

## Testing

```bash
make test                    # Run all tests
cargo test test_name         # Run specific test
RUST_LOG=debug cargo test    # With logging
```

## License

AGPL-3.0-or-later

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.
