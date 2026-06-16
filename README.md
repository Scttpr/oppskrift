# Oppskrift

A federated recipe sharing platform built with Rust and ActivityPub.

## Features

- **User Authentication**: Secure registration, login, and session management with email verification
- **Two-Factor Authentication**: TOTP-based 2FA with recovery codes
- **Recipe Management**: Create, edit, and organize recipes with ingredients, instructions, and images
- **Recipe Books**: Organize recipes into collections
- **Groups & Sharing**: Attribute-based access control (ABAC) for sharing recipes and books with groups
- **Account Settings**: Manage profile, security, and account preferences
- **Recipe Export**: Export recipes and books
- **Social Features**: Follow users, save recipes, share with activity feed
- **Federation**: ActivityPub support for cross-instance recipe sharing
- **RSS/Atom Feeds**: Subscribe to user recipes via feed readers
- **WebFinger**: Discover users across federated instances
- **oEmbed**: Rich embeds for recipes shared on other platforms
- **Rate Limiting**: Per-IP / per-account sliding-window limits on auth, uploads, search, and exports

## Tech Stack

- **Backend**: Rust with Axum 0.8
- **Database**: PostgreSQL 15+ with SQLx
- **Templates**: Askama (compile-time checked)
- **Frontend**: Server-rendered HTML + HTMX + Tailwind CSS
- **Storage**: S3-compatible (MinIO for development)
- **Federation**: activitypub-federation-rust 0.6

## Quick Start

### Prerequisites

- Rust 1.85+ (stable) — some dependencies require edition 2024
- Docker/Podman with Compose
- Tailwind CSS CLI (optional, for CSS changes)

### Development Setup

```bash
# Clone and enter directory
git clone https://github.com/scttpr/oppskrift.git
cd oppskrift

# Copy environment file
cp .env.example .env

# First-time setup (starts database and runs migrations)
make setup

# Run the app with auto-reload
make dev
```

The app will be available at http://localhost:3000

### Makefile Commands

```bash
# Development
make setup       # First-time setup (db + migrations)
make dev         # Run with auto-reload (requires cargo-watch)
make css         # Build Tailwind CSS
make css-watch   # Watch CSS for changes

# Database
make db          # Start database container
make migrate     # Run migrations
make reset-db    # Drop, recreate, and migrate

# Quality
make lint        # Run clippy + format check
make test        # Run all tests
make fmt         # Format code

# Cleanup
make clean       # Clean build artifacts and stop containers
```

## Project Structure

```
src/
├── api/          # REST/JSON API endpoints
│   ├── auth.rs       # Authentication (register, login, logout, 2FA)
│   ├── account.rs    # Account management (profile)
│   ├── users.rs      # User endpoints
│   ├── recipes.rs    # Recipe CRUD
│   ├── books.rs      # Recipe book management
│   ├── groups.rs     # Groups & ABAC sharing
│   ├── social.rs     # Follow, save, share
│   ├── activitypub.rs# Federation endpoints
│   ├── feeds.rs      # RSS/Atom feeds
│   ├── webfinger.rs  # WebFinger discovery
│   ├── oembed.rs     # oEmbed provider
│   ├── openapi.rs    # OpenAPI / Swagger UI
│   └── middleware/   # Security headers, rate limiting
├── handlers/     # HTML page handlers (recipes, books, groups,
│                 #   permissions, settings, feed, legal, users)
├── services/     # Business logic (auth, password, session, email,
│                 #   recipe, book, group, permission, totp, image, export, ...)
├── models/       # Database models
└── core/         # Shared utilities
    ├── activitypub/  # AP protocol implementation
    ├── config.rs     # Configuration validation
    ├── db.rs         # Database pool
    ├── crypto.rs     # Encryption helpers
    ├── csrf.rs       # CSRF protection
    ├── error.rs      # Error types
    ├── pagination.rs # Pagination helpers
    ├── audit.rs      # Security audit logging
    └── seeds/        # Database seed data

templates/        # Askama HTML templates
static/           # CSS, JS (HTMX vendored)
migrations/       # SQLx database migrations
tests/            # Integration and security tests
```

## API Endpoints

### REST API (`/api/v1`)

#### Authentication
- `POST /auth/register` - Register new account (requires email confirmation)
- `GET /auth/confirm-email/{token}` - Confirm email address
- `POST /auth/resend-confirmation` - Resend confirmation email
- `POST /auth/login` - Login and receive session cookie
- `POST /auth/logout` - Terminate session

#### Account
- `GET /account/profile` - Get authenticated user's profile

#### Content
- `GET/POST/PUT/DELETE /recipes` - Recipe CRUD
- `GET/POST/PUT/DELETE /books` - Recipe book CRUD
- `GET/POST/PUT/DELETE /groups` - Groups & ABAC sharing
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

Required in production (set `RUST_ENV=production` to enforce):
```bash
TOTP_ENCRYPTION_KEY=your-64-hex-chars  # For 2FA (openssl rand -hex 32)
```

Optional auth settings:
```bash
SESSION_EXPIRY_DAYS=7
LOCKOUT_DURATION_MINUTES=15
SMTP_HOST=smtp.example.com  # Required for email confirmation
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
